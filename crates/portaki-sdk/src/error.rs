//! Error types surfaced by SDK host wrappers and Wasm dispatch.
//!
//! All fallible module APIs return [`Result<T>`] — never panic across the Wasm
//! boundary. Map errors to SDUI empty states or structured logs; avoid leaking
//! raw host error strings to guest-facing copy.
//!
//! ## Contract
//!
//! | Variant | Typical cause | Module response |
//! |---------|---------------|-----------------|
//! | [`PortakiError::HostNotConfigured`] | Missing mock in tests / dev | Install [`crate::host::runtime::with_host`] |
//! | [`PortakiError::CapabilityNotAvailable`] | Plan gate or missing optional cap | Fallback UX |
//! | [`PortakiError::CredentialMissing`] | BYOK not configured for a connector | Surface setup UX / degrade connector path |
//! | [`PortakiError::Storage`] | KV/repo failure | Retry or degrade read path |
//! | [`PortakiError::Connector`] | External API failure | Show connector error state |
//! | [`PortakiError::Email`] | Content over a [`crate::limits`] cap, stay ended, send cap | Fix content / stop sending |
//! | [`PortakiError::EventLimitExceeded`] | More than [`crate::limits::EVENTS_PER_INVOCATION`] emits | Batch into fewer events |
//!
//! # Examples
//!
//! ```
//! use portaki_sdk::error::{PortakiError, Result};
//!
//! fn load_cache() -> Result<Vec<u8>> {
//!     match portaki_sdk::host::kv::get("weather.cache")? {
//!         Some(bytes) => Ok(bytes),
//!         None => Err(PortakiError::Host("cache miss".into())),
//!     }
//! }
//! ```

use thiserror::Error;

pub use crate::host::email::EmailError;

/// Result alias used throughout Portaki module code.
pub type Result<T> = std::result::Result<T, PortakiError>;

/// Error enum returned from host function wrappers and serialization boundaries.
#[derive(Debug, Error)]
pub enum PortakiError {
    /// No [`crate::host::runtime::HostBackend`] installed on the current thread.
    ///
    /// Production Wasm sets the backend inside the Extism shim; unit tests must
    /// wrap code in [`crate::host::runtime::with_host`].
    #[error("host functions not configured")]
    HostNotConfigured,

    /// Capability is not granted for the current property/plan.
    ///
    /// Prefer checking [`crate::Context::has_capability`] before calling gated
    /// host APIs to avoid this error in happy paths.
    #[error("capability not available: {0}")]
    CapabilityNotAvailable(String),

    /// Gateway rejected a host import with a machine-readable reason string.
    ///
    /// Treat as operational failure — log with `invocation_id` and surface a
    /// generic retry message to guests.
    #[error("host error: {0}")]
    Host(String),

    /// JSON serialization/deserialization failed crossing the Wasm FFI boundary.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Entity repository or KV operation failed.
    #[error("storage error: {0}")]
    Storage(String),

    /// Connector operation failed after egress from the gateway.
    #[error("connector error: {0}")]
    Connector(String),

    /// No credential configured for the requested provider id.
    ///
    /// Distinct from capability absence — the plan may allow BYOK but the host
    /// has not stored a key yet.
    #[error("credential not configured: {0}")]
    CredentialMissing(String),

    /// `host::email::send` refused — by the SDK before reaching the host, or by the host.
    ///
    /// See [`EmailError`] for the cases and their machine codes.
    #[error(transparent)]
    Email(#[from] EmailError),

    /// The gateway refused an event past [`crate::limits::EVENTS_PER_INVOCATION`] for this
    /// invocation (host code `event_limit_exceeded`).
    #[error(
        "event_limit_exceeded: at most {} gateway events per invocation",
        crate::limits::EVENTS_PER_INVOCATION
    )]
    EventLimitExceeded,
}

impl PortakiError {
    /// Host code for [`PortakiError::EventLimitExceeded`].
    pub const EVENT_LIMIT_EXCEEDED_CODE: &'static str = "event_limit_exceeded";

    /// Builds the error for a host dispatch failure `code` / `message`.
    ///
    /// Codes the SDK knows become typed variants, so a module can match on them instead of
    /// parsing strings; anything else stays [`PortakiError::Host`] as `"{code}: {message}"`.
    pub fn from_host_code(code: &str, message: &str) -> Self {
        if code == Self::EVENT_LIMIT_EXCEEDED_CODE {
            return Self::EventLimitExceeded;
        }
        if let Some(email) = EmailError::from_host_code(code) {
            return Self::Email(email);
        }
        Self::Host(format!("{code}: {message}"))
    }

    /// Re-types a [`PortakiError::Host`] whose text starts with a known host code.
    ///
    /// Backends other than the Extism one (test mocks, `portaki dev`) may still report host
    /// refusals as `Host("email_stay_ended: …")`; wrappers pass their result through here so
    /// the module sees the same variant whichever backend is installed.
    #[cfg_attr(not(any(feature = "email", test)), allow(dead_code))]
    pub(crate) fn typed(self) -> Self {
        match self {
            Self::Host(text) => {
                let (code, message) = text.split_once(':').unwrap_or((text.as_str(), ""));
                match Self::from_host_code(code.trim(), message.trim()) {
                    Self::Host(_) => Self::Host(text),
                    typed => typed,
                }
            }
            other => other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_host_codes_become_typed_variants() {
        assert!(matches!(
            PortakiError::from_host_code("email_stay_ended", "stay ended"),
            PortakiError::Email(EmailError::StayEnded)
        ));
        assert!(matches!(
            PortakiError::from_host_code("email_limit_exceeded", ""),
            PortakiError::Email(EmailError::LimitExceeded)
        ));
        assert!(matches!(
            PortakiError::from_host_code("event_limit_exceeded", ""),
            PortakiError::EventLimitExceeded
        ));
        let other = PortakiError::from_host_code("kv_quota", "full");
        assert_eq!(other.to_string(), "host error: kv_quota: full");
    }

    #[test]
    fn host_strings_carrying_a_code_are_retyped() {
        let typed = PortakiError::Host("email_limit_exceeded: 6th send".into()).typed();
        assert!(matches!(
            typed,
            PortakiError::Email(EmailError::LimitExceeded)
        ));
        let untouched = PortakiError::Host("connector_egress_failed: boom".into()).typed();
        assert_eq!(
            untouched.to_string(),
            "host error: connector_egress_failed: boom"
        );
    }
}
