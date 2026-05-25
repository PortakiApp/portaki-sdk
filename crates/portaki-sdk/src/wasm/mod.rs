//! Wasm / Extism entry points and dispatch (target `wasm32` only).

pub mod dispatch;
pub mod envelope;
pub mod registry;

pub use envelope::{WasmContextEnvelope, WasmRequestEnvelope};
pub use registry::{HandlerRegistration, WasmHandlerFn};
