//! Compile-time handler registry (`inventory`).

use serde_json::Value;

use crate::context::Context;
use crate::error::Result;

/// Type-erased wasm handler invoked by the central dispatcher.
pub type WasmHandlerFn = fn(Context, Value) -> Result<Value>;

/// One registered query, command, or surface handler.
pub struct HandlerRegistration {
    /// Manifest / runtime names that resolve to this handler.
    pub operation_names: &'static [&'static str],
    /// Handler implementation (shim around the module function).
    pub dispatch: WasmHandlerFn,
}

inventory::collect!(HandlerRegistration);

/// Finds a handler by operation name, including `render_guest_*` / `render_host_*` aliases.
pub fn find_handler(operation: &str) -> Option<&'static HandlerRegistration> {
    for registration in inventory::iter::<HandlerRegistration> {
        if registration.operation_names.contains(&operation) {
            return Some(registration);
        }
    }
    for registration in inventory::iter::<HandlerRegistration> {
        if let Some(stripped) = operation.strip_prefix("render_guest_") {
            let candidate = format!("render_{stripped}");
            if registration.operation_names.contains(&candidate.as_str()) {
                return Some(registration);
            }
        }
        if let Some(stripped) = operation.strip_prefix("render_host_") {
            let candidate = format!("render_{stripped}");
            if registration.operation_names.contains(&candidate.as_str()) {
                return Some(registration);
            }
        }
    }
    None
}
