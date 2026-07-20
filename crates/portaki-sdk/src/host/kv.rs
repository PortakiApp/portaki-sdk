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
//! - [`atomic_set`] provides compare-and-set for lightweight coordination.
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
//!     fn has_capability(&self, _: &str) -> Result<bool> { Ok(true) }
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

use crate::error::{PortakiError, Result};
use crate::host::runtime::backend;

const FORBIDDEN_SUBSTRINGS: &[&str] = &["token", "password", "secret", "credential", "auth"];

/// Reads raw bytes at `key`, or `None` when unset / expired.
pub fn get(key: &str) -> Result<Option<Vec<u8>>> {
    backend()?.kv_get(key)
}

/// Stores `value` with optional TTL in seconds.
///
/// Returns an error when `key` matches secret-like substrings — never store
/// API tokens in KV (gateway holds connector secrets).
pub fn set(key: &str, value: &[u8], ttl_seconds: Option<u32>) -> Result<()> {
    lint_key(key)?;
    backend()?.kv_set(key, value, ttl_seconds)
}

/// Deletes `key` if present.
pub fn delete(key: &str) -> Result<()> {
    backend()?.kv_delete(key)
}

/// Lists keys beginning with `prefix` (module-scoped namespace).
pub fn list(prefix: &str) -> Result<Vec<String>> {
    backend()?.kv_list(prefix)
}

/// Compare-and-set: writes `new` only when the stored value matches `expected`.
///
/// Pass `expected: None` to create only when the key is absent. Returns `true`
/// when the write succeeded.
pub fn atomic_set(key: &str, expected: Option<&[u8]>, new: &[u8]) -> Result<bool> {
    lint_key(key)?;
    let current = get(key)?;
    let matches = match (current.as_deref(), expected) {
        (Some(current), Some(expected)) => current == expected,
        (None, None) => true,
        _ => false,
    };
    if matches {
        set(key, new, None)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

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
    use super::lint_key;

    #[test]
    fn rejects_secret_like_keys() {
        assert!(lint_key("oauth_token").is_err());
        assert!(lint_key("cache.weather").is_ok());
    }
}
