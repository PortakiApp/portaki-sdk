//! `host::kv` — small ephemeral key-value storage scoped per module and property.

use crate::error::{PortakiError, Result};
use crate::host::runtime::backend;

const FORBIDDEN_SUBSTRINGS: &[&str] = &["token", "password", "secret", "credential", "auth"];

/// Returns the raw bytes stored at `key`, if any.
pub fn get(key: &str) -> Result<Option<Vec<u8>>> {
    backend()?.kv_get(key)
}

/// Stores `value` with an optional TTL in seconds.
pub fn set(key: &str, value: &[u8], ttl_seconds: Option<u32>) -> Result<()> {
    lint_key(key)?;
    backend()?.kv_set(key, value, ttl_seconds)
}

/// Deletes `key`.
pub fn delete(key: &str) -> Result<()> {
    backend()?.kv_delete(key)
}

/// Lists keys starting with `prefix`.
pub fn list(prefix: &str) -> Result<Vec<String>> {
    backend()?.kv_list(prefix)
}

/// Compare-and-set: sets `new` only when current value matches `expected`.
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
            "kv key '{key}' looks like a secret — use host::credentials instead"
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
