//! Wasm / Extism entry points and dispatch (target `wasm32` only).

pub mod dispatch;
#[cfg(target_arch = "wasm32")]
pub mod extism_host;
pub mod envelope;
pub mod registry;

pub use envelope::{WasmContextEnvelope, WasmRequestEnvelope};
pub use registry::{HandlerRegistration, WasmHandlerFn};
