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

/// What a handler is, as its attribute declared it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HandlerKind {
    /// `#[query(name = "…")]`.
    Query,
    /// `#[command(name = "…")]`.
    Command,
    /// `#[surface(guest|host, id = "…")]`.
    Surface,
}

/// One handler as its attribute declared it — native targets only.
///
/// The Wasm binary dispatches by name through [`HandlerRegistration`] and needs nothing else. A
/// native test does: to exercise every surface a module declares, it has to know which handlers
/// are surfaces, and in which shell they render. `query`, `command` and `surface` submit one of
/// these on every target but `wasm32`, so a module binary is unchanged.
///
/// Read with [`declarations`]; `portaki-test-utils`'s conformance battery is the intended reader.
pub struct HandlerDeclaration {
    /// Query, command or surface.
    pub kind: HandlerKind,
    /// The operation name (`getConfig`), or the surface id (`home.card`).
    pub name: &'static str,
    /// `guest` or `host` for a surface; empty for a query or a command.
    pub context: &'static str,
    /// The Rust function behind it (`render_home_card`).
    pub fn_name: &'static str,
    /// The same shim the Wasm entry points call: typed args from JSON, result to JSON.
    pub dispatch: WasmHandlerFn,
}

inventory::collect!(HandlerDeclaration);

/// Every handler the linked module crates declared, in no particular order.
///
/// Empty on `wasm32`, where handlers register as [`HandlerRegistration`] only. A crate is only in
/// the list once it is linked: a test binary that never names the module crate sees nothing.
pub fn declarations() -> impl Iterator<Item = &'static HandlerDeclaration> {
    inventory::iter::<HandlerDeclaration>.into_iter()
}

/// One `#[params]` shape, as JSON — native targets only, like [`HandlerDeclaration`].
///
/// The conformance battery reads a config row's fields here (see
/// [`crate::config::resolve_items`]).
pub struct ParamsDeclaration {
    /// The type name (`Step`).
    pub name: &'static str,
    /// The emitted shape (`{ "fields": [{ "name", "type", … }] }`), JSON.
    pub shape: &'static str,
}

inventory::collect!(ParamsDeclaration);

/// The `#[params]` shape of the linked type called `name`; `None` on `wasm32`.
pub fn params_shape(name: &str) -> Option<Value> {
    inventory::iter::<ParamsDeclaration>
        .into_iter()
        .find(|declaration| declaration.name == name)
        .and_then(|declaration| serde_json::from_str(declaration.shape).ok())
}
