//! Typed surface / operation catalogs for this module.
//!
//! Add `define_surface_ids!` / `define_operation_names!` / `define_event_types!`
//! entries as the module grows. See SDK docs/typed-ids.md.

use portaki_sdk::prelude::*;

/// Catalog module id (`{{MODULE_NAME}}`).
pub fn module_id() -> ModuleId {
    ModuleId::from_static("{{MODULE_NAME}}")
}
