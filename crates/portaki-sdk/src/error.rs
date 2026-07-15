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
//! | [`PortakiError::CredentialMissing`] | BYOK not configured | Call [`crate::host::credentials::request_setup`] |
//! | [`PortakiError::Storage`] | KV/repo failure | Retry or degrade read path |
//! | [`PortakiError::Connector`] | External API failure | Show connector error state |
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
}
