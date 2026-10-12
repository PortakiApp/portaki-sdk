//! `host::stay_secrets` — a small secret tied to one stay: a door code, a keypad code.
//!
//! [`super::kv`] is the wrong place for it. KV refuses secret-like keys and is not meant to hold
//! them; and a write from the host dashboard lands in the draft layer, which the guest booklet
//! does not read until the host publishes. A stay secret is neither configuration nor draft: the
//! host sets it for one stay, the guest of that stay sees it at once.
//!
//! ## Contract
//!
//! - Values are short strings (1 to 256 bytes), encrypted at rest by the platform, scoped to the
//!   property **and** the module: another module, or the same module on another property, never
//!   reads them.
//! - The platform checks the stay on every call, from the invocation — never from the module:
//!   - **guest** invocation: only the invocation's own stay ([`crate::Context::stay`]); any
//!     other id is refused;
//!   - **host** invocation: the stay must belong to the invocation's workspace and property
//!     (the module names it — e.g. `stayId` on a stay-detail surface — the orchestrator checks it).
//! - [`list`] (which stays hold a key — never values) is host-only.
//! - A secret is deleted with its stay, and 7 days after the stay's check-out at the latest. A
//!   [`set`] after that is refused.
//! - Never draft: what a host sets is what the guest reads, immediately. Revealing it is still
//!   the module's job — [`crate::reveal`] — and the plaintext must not enter a guest tree while
//!   locked.
//! - Keys: `[a-z0-9][a-z0-9_.-]{0,63}`; at most 16 keys per stay.
//!
//! Requires the `kv` permission (feature `kv`).
//!
//! ```no_run
//! use portaki_sdk::host::stay_secrets;
//! # fn main() -> portaki_sdk::error::Result<()> {
//! # let stay = uuid::Uuid::nil();
//! stay_secrets::set(stay, "door_code", "4821")?;
//! assert_eq!(stay_secrets::get(stay, "door_code")?.as_deref(), Some("4821"));
//! # Ok(())
//! # }
//! ```

use uuid::Uuid;

use crate::error::{PortakiError, Result};
use crate::host::runtime::backend;

/// The largest value the platform stores, in UTF-8 bytes.
pub const MAX_VALUE_BYTES: usize = 256;

/// The secret `key` of `stay`, or `None` when unset or expired.
pub fn get(stay: Uuid, key: &str) -> Result<Option<String>> {
    lint_key(key)?;
    backend()?.stay_secret_get(&stay.to_string(), key)
}

/// Stores `value` as the secret `key` of `stay`, replacing any previous one.
pub fn set(stay: Uuid, key: &str, value: &str) -> Result<()> {
    lint_key(key)?;
    if value.is_empty() || value.len() > MAX_VALUE_BYTES {
        return Err(PortakiError::Host(format!(
            "stay_secret_value_size: 1 to {MAX_VALUE_BYTES} bytes"
        )));
    }
    backend()?.stay_secret_set(&stay.to_string(), key, value)
}

/// Deletes the secret `key` of `stay`, if any.
pub fn delete(stay: Uuid, key: &str) -> Result<()> {
    lint_key(key)?;
    backend()?.stay_secret_delete(&stay.to_string(), key)
}

/// The stays of this property holding `key` — host invocations only. Never the values.
pub fn list(key: &str) -> Result<Vec<Uuid>> {
    lint_key(key)?;
    Ok(backend()?
        .stay_secret_list(key)?
        .iter()
        .filter_map(|id| Uuid::parse_str(id).ok())
        .collect())
}

/// The platform's key grammar, checked here so a typo fails in the module's own tests.
fn lint_key(key: &str) -> Result<()> {
    let mut chars = key.chars();
    let first_ok = chars
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    let rest_ok = chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "_.-".contains(c));
    if first_ok && rest_ok && key.len() <= 64 {
        Ok(())
    } else {
        Err(PortakiError::Host(format!(
            "stay_secret_invalid_key: '{key}'"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_follow_the_platform_grammar() {
        assert!(lint_key("door_code").is_ok());
        assert!(lint_key("parking.code-2").is_ok());
        assert!(lint_key("").is_err());
        assert!(lint_key("Door").is_err());
        assert!(lint_key("_code").is_err());
        assert!(lint_key("a/b").is_err());
        assert!(lint_key(&"a".repeat(65)).is_err());
    }

    #[test]
    fn without_a_host_nothing_is_read_or_written() {
        std::thread::spawn(|| {
            assert!(get(Uuid::nil(), "door_code").is_err());
            assert!(set(Uuid::nil(), "door_code", "1234").is_err());
        })
        .join()
        .unwrap();
    }

    #[test]
    fn a_value_is_bounded_before_it_leaves_the_module() {
        assert!(set(Uuid::nil(), "door_code", "").is_err());
        assert!(set(Uuid::nil(), "door_code", &"9".repeat(MAX_VALUE_BYTES + 1)).is_err());
    }
}
