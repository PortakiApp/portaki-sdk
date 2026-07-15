//! Opaque credential handles — secrets never enter the Wasm linear memory.
//!
//! Connectors receive a [`CredentialHandle`] reference; the Java gateway resolves the
//! handle to a vault secret at egress time. Modules audit credential access through
//! `Context::invocation_id` attached to host dispatch logs.
//!
//! ## Contract
//!
//! - [`get`] returns `Ok(None)` when BYOK is not configured — not an error.
//! - [`list`] exposes diagnostic status for host settings UI — not for guest surfaces.
//! - [`request_setup`] returns i18n-backed instructions; the host surfaces a setup flow.
//!
//! ## What modules must not assume
//!
//! - Handles are not durable identifiers — do not persist them in KV or entities.
//! - Pool credentials (`external.*.pool`) may not require BYOK — still use connectors.
//! - Cleartext tokens never cross the Extism boundary; do not log handle contents.
//!
//! # Examples
//!
//! ```ignore
//! use portaki_sdk::prelude::*;
//!
//! fn fetch_places(ctx: &HostContext) -> Result<Surface> {
//!     if host::credentials::get("google-places")?.is_none() {
//!         let setup = host::credentials::request_setup("google-places")?;
//!         // Render EmptyState with setup.message_key
//!         let _ = setup;
//!     }
//!     // Safe to call connector — gateway injects secret at egress
//!     Ok(Surface::new(/* … */))
//! }
//! ```

use serde::{Deserialize, Serialize};

use crate::error::Result;

/// Opaque reference passed to connector egress — gateway resolves to vault secret.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CredentialHandle {
    /// Provider slug (`google-places`, `mapbox`, `open-weather`, …).
    pub provider_id: String,
    /// Opaque token reference meaningful only to the gateway vault.
    pub handle: String,
}

/// BYOK configuration status for host diagnostics surfaces.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CredentialStatus {
    /// Provider slug.
    pub provider_id: String,
    /// `true` when a property-owned secret is stored and validated.
    pub configured: bool,
    /// Last successful validation timestamp (RFC3339), if known.
    pub last_validated_at: Option<String>,
}

/// Host-facing setup prompt when a module needs BYOK configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SetupInstructions {
    /// Provider the owner must configure.
    pub provider_id: String,
    /// i18n key for the setup prompt body.
    pub message_key: String,
}

/// Returns a [`CredentialHandle`] for `provider_id` when BYOK is configured.
///
/// v1 module builds return `Ok(None)` until gateway wiring is complete — guard
/// connector calls and surface setup UX when `None`.
pub fn get(provider_id: &str) -> Result<Option<CredentialHandle>> {
    let _ = provider_id;
    Ok(None)
}

/// Lists credential status rows for the current property (host diagnostics).
pub fn list() -> Result<Vec<CredentialStatus>> {
    Ok(Vec::new())
}

/// Signals that the module requires owner configuration for `provider_id`.
///
/// The returned [`SetupInstructions::message_key`] should drive an empty state
/// or settings banner in host surfaces.
pub fn request_setup(provider_id: &str) -> Result<SetupInstructions> {
    Ok(SetupInstructions {
        provider_id: provider_id.to_string(),
        message_key: format!("credential.{provider_id}.setup"),
    })
}
