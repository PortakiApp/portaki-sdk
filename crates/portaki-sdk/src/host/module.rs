//! Module install and configuration readiness snapshot.
//!
//! Surfaces that require owner setup (API keys, required config fields) should call
//! [`status`] and render an empty state when [`ModuleStatus::is_ready`] is `false`.
//! The orchestrator — not Wasm — owns enablement and config persistence.
//!
//! ## Contract
//!
//! - [`status`] is authoritative for the **current** property + workspace + module triple.
//! - `incomplete` mirrors dashboard publication readiness (required config keys empty).
//! - `missing_required_keys` lists manifest config field keys still unset.
//!
//! ## What modules must not assume
//!
//! - `active` can be `true` while `incomplete` is `true` — module runs but should block UX.
//! - This API does not mutate config; owners edit settings through host UI flows.
//! - Values may change between invocations — do not cache readiness across guest sessions.
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

use crate::error::Result;
use crate::host::runtime::backend;

/// Orchestrator snapshot of module enablement and config completeness.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
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

/// Fetches the current module status from the gateway orchestrator.
///
/// Requires an installed [`crate::host::runtime::HostBackend`] — unavailable in bare
/// unit tests without [`crate::host::runtime::with_host`].
pub fn status() -> Result<ModuleStatus> {
    backend()?.module_status()
}
