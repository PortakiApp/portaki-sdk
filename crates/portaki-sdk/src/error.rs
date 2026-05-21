//! Error types returned by SDK host wrappers.

use thiserror::Error;

/// Result alias for Portaki module code.
pub type Result<T> = std::result::Result<T, PortakiError>;

/// Errors surfaced to module authors from host function calls.
#[derive(Debug, Error)]
pub enum PortakiError {
    /// Host function backend is not configured (e.g. missing mock in tests).
    #[error("host functions not configured")]
    HostNotConfigured,

    /// Capability is not available for the current property/plan.
    #[error("capability not available: {0}")]
    CapabilityNotAvailable(String),

    /// Generic host failure with a machine-readable reason.
    #[error("host error: {0}")]
    Host(String),

    /// Serialization failure crossing the Wasm boundary.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Repository or KV operation failed.
    #[error("storage error: {0}")]
    Storage(String),

    /// Connector call failed.
    #[error("connector error: {0}")]
    Connector(String),

    /// Credential is missing for the requested provider.
    #[error("credential not configured: {0}")]
    CredentialMissing(String),
}
