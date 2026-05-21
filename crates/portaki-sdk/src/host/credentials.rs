//! `host::credentials` — opaque credential handles (never cleartext tokens).

use serde::{Deserialize, Serialize};

use crate::error::Result;

/// Opaque handle passed to connectors; gateway injects the secret at egress.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CredentialHandle {
    /// Provider id (`google-places`, `mapbox`, …).
    pub provider_id: String,
    /// Opaque token reference.
    pub handle: String,
}

/// Credential status for diagnostics UIs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CredentialStatus {
    /// Provider id.
    pub provider_id: String,
    /// Whether BYOK is configured.
    pub configured: bool,
    /// Last validation timestamp (RFC3339).
    pub last_validated_at: Option<String>,
}

/// Instructions shown to the host when a module needs setup.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SetupInstructions {
    /// Provider to configure.
    pub provider_id: String,
    /// i18n key for the setup prompt.
    pub message_key: String,
}

/// Returns a handle for `provider_id` when configured.
pub fn get(provider_id: &str) -> Result<Option<CredentialHandle>> {
    let _ = provider_id;
    Ok(None)
}

/// Lists credential status for the property (diagnostics).
pub fn list() -> Result<Vec<CredentialStatus>> {
    Ok(Vec::new())
}

/// Signals that the module needs the host to configure a provider.
pub fn request_setup(provider_id: &str) -> Result<SetupInstructions> {
    Ok(SetupInstructions {
        provider_id: provider_id.to_string(),
        message_key: format!("credential.{provider_id}.setup"),
    })
}
