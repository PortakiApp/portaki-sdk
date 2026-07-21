//! Host function runtime — thread-local backend dispatch.
//!
//! Every `host::*` wrapper resolves the active [`HostBackend`] installed for the
//! current thread. Production Wasm (target `wasm32`) uses
//! `crate::wasm::extism_host::ExtismHostBackend`; tests and `portaki dev` inject
//! mocks through [`with_host`].
//!
//! ## Contract
//!
//! - [`with_host`] sets both backend and [`crate::Context`] for the closure scope —
//!   backends are cleared when the closure returns.
//! - [`HostBackend`] methods are synchronous; async gateway work is hidden behind FFI.
//! - Default trait methods return [`crate::error::PortakiError::HostNotConfigured`] —
//!   mocks may omit overrides for unimplemented ops.
//!
//! ## What modules must not assume
//!
//! - Thread-local state does not propagate across spawned threads inside Wasm.
//! - `context_or_load` prefers thread-local context, then calls `backend.context()`.
//! - Native `cargo test` on non-wasm targets still needs `with_host` for host calls.
//!
//! # Examples
//!
//! ```no_run
//! use portaki_sdk::context::Context;
//! use portaki_sdk::host::runtime::{with_host, HostBackend};
//! use portaki_sdk::error::{PortakiError, Result};
//! use std::sync::Arc;
//!
//! struct NoopHost;
//! impl HostBackend for NoopHost {
//!     fn context(&self) -> Result<Context> { Ok(Context::default()) }
//!     fn has_capability(&self, _: &str) -> Result<bool> { Ok(false) }
//!     fn kv_get(&self, _: &str) -> Result<Option<Vec<u8>>> { Ok(None) }
//!     fn kv_set(&self, _: &str, _: &[u8], _: Option<u32>) -> Result<()> { Ok(()) }
//!     fn kv_delete(&self, _: &str) -> Result<()> { Ok(()) }
//!     fn kv_list(&self, _: &str) -> Result<Vec<String>> { Ok(vec![]) }
//!     fn i18n_translate(&self, key: &str, _: &str) -> Result<String> { Ok(key.into()) }
//!     fn log(&self, _: &str, _: &str, _: &str) -> Result<()> { Ok(()) }
//!     fn connector_call(&self, _: &str, _: &str, _: &str) -> Result<String> {
//!         Err(PortakiError::HostNotConfigured)
//!     }
//!     fn emit_event(&self, _: &str, _: &str) -> Result<()> { Ok(()) }
//! }
//!
//! with_host(Arc::new(NoopHost), Context::default(), || {
//!     assert!(!portaki_sdk::host::capabilities::has("core.images").unwrap());
//! });
//! ```

use std::cell::RefCell;
use std::sync::Arc;

use crate::context::Context;
use crate::error::{PortakiError, Result};

thread_local! {
    static HOST: RefCell<Option<Arc<dyn HostBackend>>> = const { RefCell::new(None) };
    static CTX: RefCell<Option<Context>> = const { RefCell::new(None) };
}

/// Gateway-facing host import surface implemented by Java dispatch or test mocks.
pub trait HostBackend: Send + Sync {
    /// Loads the invocation [`Context`] when not already thread-local.
    fn context(&self) -> Result<Context>;

    /// Live capability probe — see [`crate::host::capabilities::has`].
    fn has_capability(&self, id: &str) -> Result<bool>;

    /// Reads a KV key scoped to property + module.
    fn kv_get(&self, key: &str) -> Result<Option<Vec<u8>>>;

    /// Writes a KV key with optional TTL seconds.
    fn kv_set(&self, key: &str, value: &[u8], ttl_seconds: Option<u32>) -> Result<()>;

    /// Deletes a KV key.
    fn kv_delete(&self, key: &str) -> Result<()>;

    /// Lists KV keys sharing `prefix`.
    fn kv_list(&self, prefix: &str) -> Result<Vec<String>>;

    /// Resolves an i18n key with interpolated variables JSON.
    fn i18n_translate(&self, key: &str, vars_json: &str) -> Result<String>;

    /// Writes a structured log line (`level`, `message`, `fields_json`).
    fn log(&self, level: &str, message: &str, fields_json: &str) -> Result<()>;

    /// Invokes a connector; returns serialized response JSON.
    fn connector_call(
        &self,
        connector_id: &str,
        operation: &str,
        args_json: &str,
    ) -> Result<String>;

    /// Publishes a domain event with JSON payload.
    fn emit_event(&self, event_type: &str, payload_json: &str) -> Result<()>;

    /// Returns current UTC time as ISO-8601 (Wasm host dispatch).
    fn time_now_iso(&self) -> Result<String> {
        Err(PortakiError::HostNotConfigured)
    }

    /// Runs a typed repository find; returns serialized [`crate::host::repo::Page`] JSON.
    fn repo_find(&self, _entity: &str, _query_json: &str) -> Result<String> {
        Err(PortakiError::HostNotConfigured)
    }

    /// Creates a repository row; returns serialized entity JSON.
    fn repo_create(&self, _entity: &str, _entity_json: &str) -> Result<String> {
        Err(PortakiError::HostNotConfigured)
    }

    /// Deletes a repository row by id; returns whether a row was removed.
    fn repo_delete(&self, _entity: &str, _id: &str) -> Result<bool> {
        Err(PortakiError::HostNotConfigured)
    }

    /// Returns module install/config readiness from the orchestrator.
    fn module_status(&self) -> Result<crate::host::module::ModuleStatus> {
        Err(PortakiError::HostNotConfigured)
    }

    /// Lists peer modules on this property that provide `capability_id`.
    fn module_list_by_capability(
        &self,
        _capability_id: &str,
    ) -> Result<Vec<crate::host::module::ModulePeer>> {
        Err(PortakiError::HostNotConfigured)
    }
}

/// Installs `backend` and `context` for the current thread while `f` runs.
///
/// Always clears thread-local state when `f` returns — even on panic (drop order
/// is handled by running cleanup after `f()`).
pub fn with_host<F, R>(backend: Arc<dyn HostBackend>, context: Context, f: F) -> R
where
    F: FnOnce() -> R,
{
    HOST.with(|host| *host.borrow_mut() = Some(backend));
    CTX.with(|ctx| *ctx.borrow_mut() = Some(context));
    let result = f();
    HOST.with(|host| *host.borrow_mut() = None);
    CTX.with(|ctx| *ctx.borrow_mut() = None);
    result
}

pub(crate) fn backend() -> Result<Arc<dyn HostBackend>> {
    HOST.with(|host| host.borrow().clone().ok_or(PortakiError::HostNotConfigured))
}

pub(crate) fn context_or_load() -> Result<Context> {
    if let Some(ctx) = CTX.with(|cell| cell.borrow().clone()) {
        return Ok(ctx);
    }
    backend()?.context()
}
