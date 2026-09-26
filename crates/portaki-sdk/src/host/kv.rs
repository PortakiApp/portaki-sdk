//! Ephemeral key-value storage scoped to property + module.
//!
//! Use KV for small caches, feature flags, and scratch state that can be rebuilt.
//! Durable domain data belongs in [`super::repo`] entities. Requires
//! [`crate::capability::core::STORAGE`] (implicit on all modules).
//!
//! ## Contract
//!
//! - Keys are module-private — the gateway namespaces by property and module id.
//! - Values are opaque byte blobs — serialize JSON or protobuf yourself.
//! - [`set`] rejects secret-like key names — never store API tokens in KV.
//! - Per-stay data goes under [`stay_key`] (`stay:<stay_id>:<key>`): the platform deletes
//!   every key under that prefix when the stay is deleted. A key elsewhere outlives the stay.
//!
//! ## What modules must not assume
//!
//! - No durability guarantees across gateway restarts — treat as cache tier.
//! - TTL enforcement is best-effort server-side; do not rely on sub-second expiry.
//! - KV is not encrypted at rest in module memory — never store PII or secrets.
//!
//! # Examples
//!
//! ```no_run
//! use portaki_sdk::context::Context;
//! use portaki_sdk::host::runtime::{with_host, HostBackend};
//! use portaki_sdk::error::{PortakiError, Result};
//! use std::sync::Arc;
//!
//! struct NoopHost;
//! impl HostBackend for NoopHost {
//!     fn context(&self) -> Result<Context> { Ok(Context::default()) }
//!     fn kv_get(&self, _: &str) -> Result<Option<Vec<u8>>> { Ok(None) }
//!     fn kv_set(&self, _: &str, _: &[u8], _: Option<u32>) -> Result<()> { Ok(()) }
//!     fn kv_delete(&self, _: &str) -> Result<()> { Ok(()) }
//!     fn kv_list(&self, _: &str) -> Result<Vec<String>> { Ok(vec![]) }
//!     fn i18n_translate(&self, key: &str, _: &str) -> Result<String> { Ok(key.into()) }
//!     fn log(&self, _: &str, _: &str, _: &str) -> Result<()> { Ok(()) }
//!     fn connector_call(&self, _: &str, _: &str, _: &str) -> Result<String> {
//!         Err(PortakiError::HostNotConfigured)
//!     }
//!     fn emit_event(&self, _: &str, _: &str) -> Result<()> { Ok(()) }
//! }
//!
//! with_host(Arc::new(NoopHost), Context::default(), || {
//!     portaki_sdk::host::kv::set("weather.cache", b"{}", Some(300)).unwrap();
//! });
//! ```

#[cfg(feature = "kv")]
use crate::error::{PortakiError, Result};
#[cfg(feature = "kv")]
use crate::host::runtime::backend;

#[cfg(feature = "kv")]
const FORBIDDEN_SUBSTRINGS: &[&str] = &["token", "password", "secret", "credential", "auth"];

/// Reads raw bytes at `key`, or `None` when unset / expired.
#[cfg(feature = "kv")]
pub fn get(key: &str) -> Result<Option<Vec<u8>>> {
    backend()?.kv_get(key)
}

/// Stores `value` with optional TTL in seconds.
///
/// Returns an error when `key` matches secret-like substrings — never store
/// API tokens in KV (gateway holds connector secrets).
#[cfg(feature = "kv")]
pub fn set(key: &str, value: &[u8], ttl_seconds: Option<u32>) -> Result<()> {
    lint_key(key)?;
    backend()?.kv_set(key, value, ttl_seconds)
}

/// Deletes `key` if present.
#[cfg(feature = "kv")]
pub fn delete(key: &str) -> Result<()> {
    backend()?.kv_delete(key)
}

/// Lists keys beginning with `prefix` (module-scoped namespace).
#[cfg(feature = "kv")]
pub fn list(prefix: &str) -> Result<Vec<String>> {
    backend()?.kv_list(prefix)
}

/// The key of `key` for one stay: `stay:<stay_id>:<key>`.
///
/// The platform owns this prefix — when the stay is deleted, every key under
/// `stay:<stay_id>:` is deleted with it, for every module of the property. Keep
/// anything tied to a guest (answers, reviews, drafts) under it.
pub fn stay_key(stay_id: uuid::Uuid, key: &str) -> String {
    format!("stay:{stay_id}:{key}")
}

#[cfg(feature = "kv")]
fn lint_key(key: &str) -> Result<()> {
    let lower = key.to_ascii_lowercase();
    if FORBIDDEN_SUBSTRINGS
        .iter()
        .any(|fragment| lower.contains(fragment))
    {
        return Err(PortakiError::Host(format!(
            "kv key '{key}' looks like a secret — do not store credentials in KV"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{lint_key, stay_key};

    #[test]
    fn stay_key_is_under_the_platform_prefix() {
        let stay = uuid::Uuid::nil();
        assert_eq!(
            stay_key(stay, "review"),
            "stay:00000000-0000-0000-0000-000000000000:review"
        );
    }

    #[test]
    fn rejects_secret_like_keys() {
        assert!(lint_key("oauth_token").is_err());
        assert!(lint_key("cache.weather").is_ok());
    }
}
