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
//!
//! # Translated text
//!
//! A field typed [`I18nText`](crate::contracts::i18n::I18nText) is `localized`: the platform
//! keeps one text per language, and a save from the host form writes the host's language only.
//! A list whose rows hold `I18nText` fields is `structured` with an `item` saying which sub-keys
//! are translated (and which one identifies a row): put `#[portaki_sdk::params]` on the row type,
//! whose fields the config macro cannot see — `portaki build` reads them there. A row field marked
//! `#[field(secret)]` is `item.secret`: encrypted at rest, and kept when a save sends it back empty
//! or masked, like a `secret` config field.
//!
//! ```
//! use portaki_sdk::contracts::i18n::I18nText;
//!
//! #[portaki_sdk::params]
//! #[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
//! #[serde(default)] // a row the host saved half-filled still reads
//! pub struct Step {
//!     pub id: String,          // → item.id, the row survives a reorder or a removal
//!     pub title: I18nText,     // → item.localized
//!     pub note: String,
//!     #[field(secret)]
//!     pub door_code: String,   // → item.secret
//! }
//!
//! #[portaki_sdk::config]
//! #[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
//! pub struct Config {
//!     #[field(label = "config.welcome")]
//!     pub welcome: I18nText, // → "type": "localized"
//!     #[field(label = "config.steps")]
//!     pub steps: Vec<Step>,  // → "item": { "id": "id", "localized": ["title"], "secret": ["door_code"] }
//! }
//!
//! let ctx = portaki_sdk::context::Context {
//!     locale: "en-US".into(),
//!     module_config: Some(serde_json::json!({
//!         "welcome": { "fr": "Bienvenue", "en": "Welcome" },
//!         "steps": [{ "id": "a", "title": "Portail" }],
//!     })),
//!     ..Default::default()
//! };
//! let config = Config::load(&ctx).unwrap();
//! // In the host form: the editor's language, or the fallback.
//! assert_eq!(config.welcome.host_value(&ctx), "Welcome");
//! assert_eq!(config.steps[0].title.host_value(&ctx), "Portail");
//! ```

use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

use crate::context::Context;
use crate::error::{PortakiError, Result};

/// The KV key a module kept its config under before the platform held it.
pub const LEGACY_KV_KEY: &str = "config";

/// The `config.fields` one `#[portaki_sdk::config]` emitted, as JSON — native targets only.
///
/// Rows still carry `itemType`, the row type [`resolve_items`] replaces by `item`. The
/// conformance battery of `portaki-test-utils` reads it to hold the manifest to the code.
pub struct ConfigDeclaration {
    /// The `configField` array, JSON.
    pub fields: &'static str,
}

inventory::collect!(ConfigDeclaration);

/// Every `#[portaki_sdk::config]` of the linked crates; empty on `wasm32`.
pub fn declarations() -> impl Iterator<Item = &'static ConfigDeclaration> {
    inventory::iter::<ConfigDeclaration>.into_iter()
}

/// Replaces each field's `itemType` by `item`: the row's `I18nText` fields become
/// `item.localized`, its `#[field(secret)]` fields `item.secret`, and `item.id` is the
/// `#[field(item_id = "…")]` given, else a field named `id`.
///
/// `shape_of` gives the `#[params]` shape of a type by name (`{ "fields": [{ "name", "type",
/// "ref" }] }`). A row without one keeps only an explicit `item_id`; a row with neither translated
/// fields nor an id gets no `item`. Shared by `portaki build` and the conformance battery.
pub fn resolve_items(fields: &mut [Value], shape_of: impl Fn(&str) -> Option<Value>) {
    for field in fields.iter_mut().filter_map(Value::as_object_mut) {
        let Some(row) = field.remove("itemType") else {
            continue;
        };
        let shape = row.as_str().and_then(&shape_of);
        let row_fields = shape
            .as_ref()
            .and_then(|shape| shape["fields"].as_array())
            .into_iter()
            .flatten();
        let mut localized = Vec::new();
        let mut secret = Vec::new();
        let mut has_id = false;
        for row_field in row_fields {
            let name = row_field["name"].as_str().unwrap_or_default();
            has_id |= name == "id";
            if row_field["ref"] == "I18nText" {
                localized.push(Value::from(name));
            }
            if row_field["secret"] == true {
                secret.push(Value::from(name));
            }
        }
        let id = field
            .get("item")
            .and_then(|item| item.get("id"))
            .cloned()
            .or_else(|| has_id.then(|| Value::from("id")));
        if localized.is_empty() && secret.is_empty() && id.is_none() {
            continue;
        }
        let mut item = Map::new();
        if let Some(id) = id {
            item.insert("id".into(), id);
        }
        item.insert("localized".into(), Value::Array(localized));
        if !secret.is_empty() {
            item.insert("secret".into(), Value::Array(secret));
        }
        field.insert("item".into(), Value::Object(item));
    }
}

/// The configuration of this install, deserialized.
///
/// Reads [`Context::module_config`]; when it is `None` (no `moduleConfig` sent), the KV key
/// [`LEGACY_KV_KEY`]. `Some({})` is an empty config, never the KV. A missing key takes its `Default` (with `#[serde(default)]`, which `#[config]` adds) and `null` reads as
/// missing. A config that does not deserialize is [`PortakiError::Storage`] — never a default,
/// which the next save would write over what the host had.
pub fn load<T: DeserializeOwned>(ctx: &Context) -> Result<T> {
    load_mapped(ctx, std::convert::identity)
}

/// [`load`], with the KV blob read through `legacy` first — what `#[config(legacy = …)]`
/// generates. `moduleConfig` is never mapped: the platform holds the declared keys already.
pub fn load_mapped<T: DeserializeOwned>(ctx: &Context, legacy: fn(Value) -> Value) -> Result<T> {
    let raw = match &ctx.module_config {
        Some(config) => config.clone(),
        None => legacy_config_mapped(legacy)?,
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

/// [`legacy_config`] through `legacy`, which maps an old blob onto the declared keys; `null`
/// (nothing in KV) is not mapped. What `legacyConfig` answers with `#[config(legacy = …)]`.
pub fn legacy_config_mapped(legacy: fn(Value) -> Value) -> Result<Value> {
    Ok(match legacy_config()? {
        Value::Null => Value::Null,
        raw => legacy(raw),
    })
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
    fn items_come_from_the_row_shape() {
        let shape = json!({ "fields": [
            { "name": "id", "type": "string" },
            { "name": "title", "type": "ref", "ref": "I18nText" },
            { "name": "place", "type": "ref", "ref": "I18nText", "required": false },
            { "name": "endsAt", "type": "string" },
            { "name": "url", "type": "string", "secret": true },
        ] });
        let mut fields = vec![
            json!({ "key": "events", "type": "structured", "itemType": "Event" }),
            json!({ "key": "spots", "type": "structured", "itemType": "Event", "item": { "id": "slug" } }),
            json!({ "key": "raw", "type": "structured", "itemType": "Unknown" }),
            json!({ "key": "keyed", "type": "structured", "itemType": "Unknown", "item": { "id": "key" } }),
            json!({ "key": "welcome", "type": "localized" }),
        ];
        resolve_items(&mut fields, |name| (name == "Event").then(|| shape.clone()));
        assert_eq!(
            fields,
            vec![
                json!({ "key": "events", "type": "structured",
                        "item": { "id": "id", "localized": ["title", "place"], "secret": ["url"] } }),
                json!({ "key": "spots", "type": "structured",
                        "item": { "id": "slug", "localized": ["title", "place"], "secret": ["url"] } }),
                json!({ "key": "raw", "type": "structured" }),
                json!({ "key": "keyed", "type": "structured", "item": { "id": "key", "localized": [] } }),
                json!({ "key": "welcome", "type": "localized" }),
            ]
        );
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
