//! Module install readiness and peer discovery by capability.
//!
//! A guest surface does not call [`status`] itself: `#[surface(guest, …)]` does, and renders the
//! SDK's inactive or incomplete state when [`ModuleStatus::is_ready`] is `false` (see
//! [`crate::guest_shell`]). A host surface that needs owner setup may still read it.
//! The orchestrator — not Wasm — owns enablement and config persistence.
//!
//! ## Contract
//!
//! - [`status`] is authoritative for the **current** property + workspace + module triple.
//! - `incomplete` mirrors dashboard publication readiness (required config keys empty).
//! - `missing_required_keys` lists manifest config field keys still unset.
//! - [`list_by_capability`] returns peer modules installed on the same property that
//!   declare `capabilities.provided` containing the requested id (e.g. `access.smart_lock`).
//! - [`ModulePeer::shared`] carries the provider's settings it declared under `shares` for that
//!   capability (`#[capability(provided, id = "…", shares = ["reveal_policy"])]`), keyed by
//!   config key — so a consumer can show the provider's value read-only instead of asking the
//!   host twice.
//!
//! ## What modules must not assume
//!
//! - `active` can be `true` while `incomplete` is `true` — module runs but should block UX.
//! - This API does not mutate config; owners edit settings through host UI flows.
//! - Values may change between invocations — do not cache readiness across guest sessions.
//! - [`ModulePeer::shared`] holds the provider's **published** values, never its draft: a host
//!   editing the provider sees the consumer catch up only once they publish.
//! - A shared key may be absent — never set, empty, or no longer declared by the provider's
//!   installed version. Fall back to the module's own setting.
//! - It never holds a secret: `portaki check` refuses to share a secret field or a list with
//!   secret sub-keys, and the platform drops one again when serving.
//!
//! # Examples
//!
//! ```ignore
//! use portaki_sdk::prelude::*;
//!
//! fn render_settings(ctx: HostContext) -> Result<Surface> {
//!     let status = host::module::status()?;
//!     if !status.is_ready() {
//!         return Ok(/* EmptyState with missing keys */);
//!     }
//!     Ok(/* normal settings surface */)
//! }
//! ```

use serde::Deserialize;

#[cfg(feature = "platform")]
use crate::capability::CapabilityId;
#[cfg(feature = "platform")]
use crate::error::Result;
#[cfg(feature = "platform")]
use crate::host::runtime::backend;
use crate::ids::ModuleId;

/// Orchestrator snapshot of module enablement and config completeness.
#[portaki_sdk_macros::wire]
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ModuleStatus {
    /// Module is enabled on this property.
    pub active: bool,
    /// Module is enabled at workspace level (property toggle allowed).
    pub workspace_enabled: bool,
    /// Required config fields are missing — same rule as dashboard publication readiness.
    pub incomplete: bool,
    /// Manifest declares a config schema with at least one field.
    pub requires_config: bool,
    /// Config keys still empty among `required` manifest fields.
    #[serde(default)]
    pub missing_required_keys: Vec<String>,
}

impl ModuleStatus {
    /// Returns `true` when the module may render full UX without a setup empty state.
    ///
    /// Requires property + workspace enablement and complete required config.
    pub fn is_ready(&self) -> bool {
        self.active && self.workspace_enabled && !self.incomplete
    }
}

/// Peer module installed on the current property that provides a capability.
#[portaki_sdk_macros::wire]
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ModulePeer {
    /// Catalog / install module id (e.g. `nuki`).
    pub module_id: ModuleId,
    /// Localized display name when available (fallback: module id).
    #[serde(default)]
    pub display_name: String,
    /// The provider's published config values for the keys it `shares` for this capability,
    /// by config key. Empty or missing keys are left out; never secrets.
    #[serde(default)]
    pub shared: serde_json::Map<String, serde_json::Value>,
}

/// Fetches the current module status from the gateway orchestrator.
///
/// Requires an installed [`crate::host::runtime::HostBackend`] — unavailable in bare
/// unit tests without [`crate::host::runtime::with_host`].
#[cfg(feature = "platform")]
pub fn status() -> Result<ModuleStatus> {
    backend()?.module_status()
}

/// Lists workspace-enabled, property-active peer modules that provide `capability_id`.
///
/// Provider modules declare the id under `capabilities.provided` in their SDK manifest.
/// Pass [`crate::CapabilityId`] / [`crate::contracts::smart_lock::CAPABILITY`].
#[cfg(feature = "platform")]
pub fn list_by_capability(capability_id: CapabilityId) -> Result<Vec<ModulePeer>> {
    backend()?.module_list_by_capability(capability_id.as_str())
}

#[cfg(test)]
mod tests {
    use super::ModulePeer;
    use serde_json::json;

    #[test]
    fn a_peer_reads_its_shared_settings() {
        let peer: ModulePeer = serde_json::from_value(json!({
            "moduleId": "provider",
            "displayName": "Provider",
            "shared": { "reveal_policy": "before_arrival", "reveal_hours": 48 }
        }))
        .unwrap();
        assert_eq!(peer.shared["reveal_policy"], "before_arrival");
        assert_eq!(peer.shared["reveal_hours"], 48);
    }

    /// A platform that predates `shared` still answers.
    #[test]
    fn a_peer_without_shared_settings_reads_empty() {
        let peer: ModulePeer = serde_json::from_value(json!({ "moduleId": "provider" })).unwrap();
        assert!(peer.shared.is_empty());
    }
}
