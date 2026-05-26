//! Central dispatch for `portaki_query` / `portaki_command`.

use std::sync::Arc;

use crate::error::{PortakiError, Result};
use crate::host::runtime::{with_host, HostBackend};
use crate::wasm::envelope::WasmRequestEnvelope;
use crate::wasm::registry;

#[cfg(target_arch = "wasm32")]
use crate::wasm::extism_host::ExtismHostBackend;

/// In-wasm host backend placeholder for non-Extism test builds.
#[cfg(not(target_arch = "wasm32"))]
struct WasmHostBackend;

#[cfg(not(target_arch = "wasm32"))]
impl HostBackend for WasmHostBackend {
    fn context(&self) -> Result<crate::context::Context> {
        Err(PortakiError::HostNotConfigured)
    }

    fn has_capability(&self, _id: &str) -> Result<bool> {
        Ok(false)
    }

    fn kv_get(&self, _key: &str) -> Result<Option<Vec<u8>>> {
        Err(PortakiError::HostNotConfigured)
    }

    fn kv_set(&self, _key: &str, _value: &[u8], _ttl_seconds: Option<u32>) -> Result<()> {
        Err(PortakiError::HostNotConfigured)
    }

    fn kv_delete(&self, _key: &str) -> Result<()> {
        Err(PortakiError::HostNotConfigured)
    }

    fn kv_list(&self, _prefix: &str) -> Result<Vec<String>> {
        Err(PortakiError::HostNotConfigured)
    }

    fn i18n_translate(&self, key: &str, _vars_json: &str) -> Result<String> {
        Ok(key.to_string())
    }

    fn log(&self, _level: &str, _message: &str, _fields_json: &str) -> Result<()> {
        Ok(())
    }

    fn connector_call(
        &self,
        _connector_id: &str,
        _operation: &str,
        _args_json: &str,
    ) -> Result<String> {
        Err(PortakiError::HostNotConfigured)
    }

    fn emit_event(&self, _event_type: &str, _payload_json: &str) -> Result<()> {
        Ok(())
    }
}

fn wasm_host_backend() -> Arc<dyn HostBackend> {
    #[cfg(target_arch = "wasm32")]
    {
        return Arc::new(ExtismHostBackend);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Arc::new(WasmHostBackend)
    }
}

fn dispatch_envelope(input: &str) -> Result<String> {
    let envelope: WasmRequestEnvelope = serde_json::from_str(input)
        .map_err(|e| PortakiError::Host(format!("wasm_envelope_parse_failed: {e}")))?;
    let operation = envelope.operation_name()?.to_string();
    let registration = registry::find_handler(&operation)
        .ok_or_else(|| PortakiError::Host(format!("wasm_handler_not_found: {operation}")))?;
    let ctx = envelope.to_context(&operation)?;
    let params = envelope.params;
    let backend = wasm_host_backend();
    let result = with_host(backend, ctx.clone(), || {
        (registration.dispatch)(ctx, params)
    })?;
    serde_json::to_string(&result)
        .map_err(|e| PortakiError::Host(format!("wasm_result_serialize_failed: {e}")))
}

/// Dispatches a JSON envelope from the host (`portaki_query`).
pub fn dispatch_query_json(input: &str) -> Result<String> {
    dispatch_envelope(input)
}

/// Dispatches a command envelope (`portaki_command`).
pub fn dispatch_command_json(input: &str) -> Result<String> {
    dispatch_envelope(input)?;
    Ok(String::new())
}
