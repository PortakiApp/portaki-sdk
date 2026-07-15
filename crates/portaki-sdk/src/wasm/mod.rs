//! Wasm / Extism integration — entry points, dispatch, and host import ABI.
//!
//! This module is compiled into every module artifact. On `wasm32` it exposes
//! the Extism exports the Java gateway invokes; on other targets the symbols exist
//! for unit tests and native `cargo test` linking.
//!
//! ## ABI overview
//!
//! ```text
//! Host (Java)                         Guest (Wasm / Extism)
//! ─────────────────                   ───────────────────────
//! portaki_query(envelope)      →      registry::find_handler → handler shim
//! portaki_command(envelope)    →      with_host(ExtismHostBackend, ctx, …)
//! portaki_host_dispatch(json)  ←      host::runtime::HostBackend methods
//! ```
//!
//! - **Inbound**: [`envelope::WasmRequestEnvelope`] JSON carries operation name,
//!   params, and a [`envelope::WasmContextEnvelope`] subset mapped to [`crate::Context`].
//! - **Outbound** (wasm32 only): `extism_host::ExtismHostBackend` wraps every
//!   `host::*` call as `{ "op": "<namespace>.<method>", "args": { ... } }`
//!   dispatched to Java via `portaki_host_dispatch`.
//!
//! ## Contract
//!
//! - Handler registration is static via `inventory` — see [`registry::HandlerRegistration`].
//! - Query/command handlers return JSON [`serde_json::Value`] serialized back to the host.
//! - Surface renders return SDUI JSON from the handler shim (not through `portaki_query`).
//!
//! ## What modules must not assume
//!
//! - `portaki_host_dispatch` latency is unbounded — avoid chatty host loops in hot paths.
//! - Context in the envelope may omit fields the gateway will add later — use defaults.
//! - Non-wasm32 builds use stub backends; always gate integration tests behind `with_host`.
//!
//! # Examples
//!
//! Module authors register handlers through proc-macros — manual registration is rare:
//!
//! ```ignore
//! use portaki_sdk::prelude::*;
//!
//! #[query(name = "list_items")]
//! fn list_items(ctx: Context, params: serde_json::Value) -> Result<serde_json::Value> {
//!     let _ = (ctx, params);
//!     Ok(serde_json::json!([]))
//! }
//! ```

pub mod dispatch;
pub mod envelope;
#[cfg(target_arch = "wasm32")]
pub mod extism_host;
pub mod registry;

pub use envelope::{WasmContextEnvelope, WasmRequestEnvelope};
pub use registry::{HandlerRegistration, WasmHandlerFn};
