//! Host function runtime dispatch (in-process mock or Wasm imports).

use std::cell::RefCell;
use std::sync::Arc;

use crate::context::Context;
use crate::error::{PortakiError, Result};

thread_local! {
    static HOST: RefCell<Option<Arc<dyn HostBackend>>> = const { RefCell::new(None) };
    static CTX: RefCell<Option<Context>> = const { RefCell::new(None) };
}

/// Backend implemented by the gateway (production) or test mocks.
pub trait HostBackend: Send + Sync {
    /// Returns the current invocation context.
    fn context(&self) -> Result<Context>;

    /// Capability probe.
    fn has_capability(&self, id: &str) -> Result<bool>;

    /// KV get.
    fn kv_get(&self, key: &str) -> Result<Option<Vec<u8>>>;

    /// KV set.
    fn kv_set(&self, key: &str, value: &[u8], ttl_seconds: Option<u32>) -> Result<()>;

    /// KV delete.
    fn kv_delete(&self, key: &str) -> Result<()>;

    /// KV list keys by prefix.
    fn kv_list(&self, prefix: &str) -> Result<Vec<String>>;

    /// i18n translate.
    fn i18n_translate(&self, key: &str, vars_json: &str) -> Result<String>;

    /// Structured log line.
    fn log(&self, level: &str, message: &str, fields_json: &str) -> Result<()>;

    /// Connector invocation (serialized args/response JSON).
    fn connector_call(
        &self,
        connector_id: &str,
        operation: &str,
        args_json: &str,
    ) -> Result<String>;

    /// Emit a domain event.
    fn emit_event(&self, event_type: &str, payload_json: &str) -> Result<()>;

    /// Current UTC time as ISO-8601 (Wasm host dispatch).
    fn time_now_iso(&self) -> Result<String> {
        Err(PortakiError::HostNotConfigured)
    }

    /// Typed repository find (JSON page payload).
    fn repo_find(&self, _entity: &str, _query_json: &str) -> Result<String> {
        Err(PortakiError::HostNotConfigured)
    }

    /// Typed repository create (JSON entity payload).
    fn repo_create(&self, _entity: &str, _entity_json: &str) -> Result<String> {
        Err(PortakiError::HostNotConfigured)
    }

    /// Typed repository delete by id.
    fn repo_delete(&self, _entity: &str, _id: &str) -> Result<bool> {
        Err(PortakiError::HostNotConfigured)
    }

    /// Current module install / config readiness (orchestrator source of truth).
    fn module_status(&self) -> Result<crate::host::module::ModuleStatus> {
        Err(PortakiError::HostNotConfigured)
    }
}

/// Installs a host backend for the current thread (used by tests and `portaki dev`).
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
