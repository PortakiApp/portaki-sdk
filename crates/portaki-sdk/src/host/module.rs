//! `host::module` — current property-module status (install + config readiness).

use serde::Deserialize;

use crate::error::Result;
use crate::host::runtime::backend;

/// Snapshot of the module row / config completeness for the current Wasm invocation.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleStatus {
    /// Module is enabled on this property.
    pub active: bool,
    /// Module is enabled at workspace level.
    pub workspace_enabled: bool,
    /// Required config fields are missing (same rule as dashboard publication readiness).
    pub incomplete: bool,
    /// Manifest declares a config schema with fields.
    pub requires_config: bool,
    /// Keys of required fields that are still empty.
    #[serde(default)]
    pub missing_required_keys: Vec<String>,
}

impl ModuleStatus {
    /// True when the module can run without a config EmptyState.
    pub fn is_ready(&self) -> bool {
        self.active && self.workspace_enabled && !self.incomplete
    }
}

/// Returns the authoritative status for the current module / property / workspace.
pub fn status() -> Result<ModuleStatus> {
    backend()?.module_status()
}
