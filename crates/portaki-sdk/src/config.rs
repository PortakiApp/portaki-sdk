//! Host configuration held by the platform — the runtime half of `#[portaki_sdk::config]`.
//!
//! A module declares its settings on a struct; the platform validates what the host saves,
//! encrypts secrets, and hands the result back on every invocation as `context.moduleConfig`
//! ([`Context::module_config`]). The module writes no `updateConfig`, no readiness check.
//!
//! Before the platform held it, a module kept its config in its own KV under [`LEGACY_KV_KEY`].
//! The platform imports that blob once, through the generated `legacyConfig` query. A runtime
//! that does not hold the config yet sends no `moduleConfig` at all (`None`): only then does
//! [`load`] read the KV key. `moduleConfig: {}` is a real, empty config — the KV is not read.
//!
//! ```
//! use portaki_sdk::context::Context;
//!
//! #[portaki_sdk::config]
//! #[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
//! pub struct Config {
//!     #[field(required, label = "config.ssid")]
//!     pub ssid: String,
//! }
//!
//! let ctx = Context {
//!     module_config: Some(serde_json::json!({ "ssid": "Villa-Azur" })),
//!     ..Context::default()
//! };
//! assert_eq!(Config::load(&ctx).unwrap().ssid, "Villa-Azur");
//! ```

use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

use crate::context::Context;
use crate::error::{PortakiError, Result};

/// The KV key a module kept its config under before the platform held it.
pub const LEGACY_KV_KEY: &str = "config";

/// The configuration of this install, deserialized.
///
/// Reads [`Context::module_config`]; when it is `None` (no `moduleConfig` sent), the KV key
/// [`LEGACY_KV_KEY`]. `Some({})` is an empty config, never the KV. A missing key takes its `Default` (with `#[serde(default)]`, which `#[config]` adds) and `null` reads as
/// missing. A config that does not deserialize is [`PortakiError::Storage`] — never a default,
/// which the next save would write over what the host had.
pub fn load<T: DeserializeOwned>(ctx: &Context) -> Result<T> {
    let raw = match &ctx.module_config {
        Some(config) => config.clone(),
        None => legacy_config()?,
    };
    let mut object = match raw {
        Value::Null => Map::new(),
        Value::Object(object) => object,
        other => return Err(unreadable(format!("expected an object, got {other}"))),
    };
    object.retain(|_, value| !value.is_null());
    serde_json::from_value(Value::Object(object)).map_err(|error| unreadable(error.to_string()))
}

/// The raw JSON of the KV key [`LEGACY_KV_KEY`], or `null` — what `legacyConfig` answers.
///
/// Always `null` without the `kv` feature: a module that has no KV kept nothing there.
pub fn legacy_config() -> Result<Value> {
    #[cfg(feature = "kv")]
    {
        let Some(bytes) = crate::host::kv::get(LEGACY_KV_KEY)? else {
            return Ok(Value::Null);
        };
        serde_json::from_slice(&bytes)
            .map_err(|error| unreadable(format!("KV `{LEGACY_KV_KEY}` is not JSON: {error}")))
    }
    #[cfg(not(feature = "kv"))]
    Ok(Value::Null)
}

fn unreadable(reason: String) -> PortakiError {
    PortakiError::Storage(format!("config_unreadable: {reason}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::runtime::{with_host, HostBackend};
    use serde::Deserialize;
    use serde_json::json;
    use std::sync::Arc;

    #[derive(Debug, Default, PartialEq, Deserialize)]
    #[serde(default)]
    struct Config {
        ssid: String,
        guests: u32,
    }

    struct Kv(Option<&'static [u8]>);
    impl HostBackend for Kv {
        fn context(&self) -> Result<Context> {
            Ok(Context::default())
        }
        fn kv_get(&self, key: &str) -> Result<Option<Vec<u8>>> {
            assert_eq!(key, LEGACY_KV_KEY);
            Ok(self.0.map(<[u8]>::to_vec))
        }
        fn kv_set(&self, _: &str, _: &[u8], _: Option<u32>) -> Result<()> {
            Ok(())
        }
        fn kv_delete(&self, _: &str) -> Result<()> {
            Ok(())
        }
        fn kv_list(&self, _: &str) -> Result<Vec<String>> {
            Ok(Vec::new())
        }
        fn i18n_translate(&self, key: &str, _: &str) -> Result<String> {
            Ok(key.into())
        }
        fn log(&self, _: &str, _: &str, _: &str) -> Result<()> {
            Ok(())
        }
        fn connector_call(&self, _: &str, _: &str, _: &str) -> Result<String> {
            Err(PortakiError::HostNotConfigured)
        }
        fn emit_event(&self, _: &str, _: &str) -> Result<()> {
            Ok(())
        }
    }

    fn load_with(module_config: Option<Value>, kv: Option<&'static [u8]>) -> Result<Config> {
        let ctx = Context {
            module_config,
            ..Context::default()
        };
        with_host(Arc::new(Kv(kv)), ctx.clone(), || load(&ctx))
    }

    #[test]
    fn module_config_wins_over_kv() {
        let config = load_with(
            Some(json!({ "ssid": "A", "guests": null })),
            Some(br#"{"ssid":"B"}"#),
        );
        assert_eq!(
            config.unwrap(),
            Config {
                ssid: "A".into(),
                guests: 0
            }
        );
    }

    #[test]
    fn without_module_config_the_kv_is_read_then_defaults() {
        let from_kv = load_with(None, Some(br#"{"ssid":"B","guests":4}"#)).unwrap();
        assert_eq!(from_kv.ssid, "B");
        assert_eq!(from_kv.guests, 4);
        assert_eq!(load_with(None, None).unwrap(), Config::default());
    }

    /// Une config vidée par l'hôte reste vide : l'ancien KV ne revient pas.
    #[test]
    fn an_empty_module_config_is_an_empty_config_and_ignores_kv() {
        let kv = Some(&br#"{"ssid":"old","guests":4}"#[..]);
        assert_eq!(load_with(Some(json!({})), kv).unwrap(), Config::default());
        assert_eq!(load_with(Some(Value::Null), kv).unwrap(), Config::default());
    }

    #[test]
    fn an_unreadable_config_is_an_error_not_a_default() {
        for (module_config, kv) in [
            (Some(json!({ "guests": "four" })), None),
            (Some(json!("ssid")), None),
            (None, Some(&b"not json"[..])),
            (None, Some(&br#"{"guests":-1}"#[..])),
        ] {
            let error = load_with(module_config.clone(), kv).unwrap_err();
            assert!(
                error.to_string().contains("config_unreadable"),
                "{module_config:?}: {error}"
            );
        }
    }

    #[test]
    fn legacy_config_is_the_raw_kv_blob() {
        let ctx = Context::default();
        let raw = with_host(Arc::new(Kv(Some(br#"{"old":true}"#))), ctx.clone(), || {
            legacy_config()
        });
        assert_eq!(raw.unwrap(), json!({ "old": true }));
        let none = with_host(Arc::new(Kv(None)), ctx, legacy_config);
        assert_eq!(none.unwrap(), Value::Null);
    }
}
