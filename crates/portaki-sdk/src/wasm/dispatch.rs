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

/// The query the platform calls for guest-email variables — see [`crate::email`].
const EMAIL_CONTEXT_QUERY: &str = "emailContext";

fn dispatch_envelope(input: &str) -> Result<String> {
    let envelope: WasmRequestEnvelope = serde_json::from_str(input)
        .map_err(|e| PortakiError::Host(format!("wasm_envelope_parse_failed: {e}")))?;
    let operation = envelope.operation_name()?.to_string();
    let ctx = envelope.to_context(&operation)?;
    // `emailContext` hands back the access codes of an arrival email. The platform is the only
    // caller, and it calls on a stay (guest caller); a host call, or one without a stay, is
    // refused here, before the module — it does not rest on the platform's guard alone.
    if operation == EMAIL_CONTEXT_QUERY && ctx.guest.is_none() {
        return Err(PortakiError::Host(format!(
            "operation_not_host_callable: {EMAIL_CONTEXT_QUERY} is called by the platform for a stay"
        )));
    }
    let registration = registry::find_handler(&operation)
        .ok_or_else(|| PortakiError::Host(format!("wasm_handler_not_found: {operation}")))?;
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

/// Dispatches a command envelope (`portaki_command`), and hands back its result.
///
/// Same JSON as a query's: a command that returns `()` answers `null`, and the host treats that
/// as no result. Before this, the result was dropped here — `getGuestCredential` could not hand
/// a code to anyone, and a `unlock` that fell back to the keypad could not say so.
pub fn dispatch_command_json(input: &str) -> Result<String> {
    dispatch_envelope(input)
}

#[cfg(test)]
mod tests {
    use super::{dispatch_command_json, dispatch_query_json};
    use crate::wasm::registry::HandlerRegistration;

    fn email_context(context: &str) -> String {
        let envelope = format!(
            r#"{{"query":"emailContext","params":{{"templateKey":"arrival"}},"context":{{"moduleId":"access-guide","moduleVersion":"1.0.0","propertyId":"790f16ef-4dbb-4295-aa7d-6e0e0ac82ba2"{context}}}}}"#
        );
        dispatch_query_json(&envelope)
            .expect_err("refused")
            .to_string()
    }

    /// Host, or no stay: refused before looking the handler up — so before the module.
    #[test]
    fn email_context_is_refused_outside_the_platform_stay_path() {
        let stay = r#","stayId":"a1b2c3d4-e5f6-7890-abcd-ef1234567890""#;
        for context in [
            format!(r#"{stay},"caller":"host""#),
            r#","caller":"guest""#.to_string(),
            String::new(),
        ] {
            assert!(
                email_context(&context).contains("operation_not_host_callable"),
                "{context}"
            );
        }
        // The platform's stay path gets past the guard (here, for want of a registered handler).
        assert!(email_context(&format!(r#"{stay},"caller":"guest""#))
            .contains("wasm_handler_not_found"));
    }

    inventory::submit! {
        HandlerRegistration {
            operation_names: &["testUnlock"],
            dispatch: |_ctx, _params| Ok(serde_json::json!({ "guest_notice": "Tapez 1234" })),
        }
    }

    inventory::submit! {
        HandlerRegistration {
            operation_names: &["testSubmit"],
            dispatch: |_ctx, _params| Ok(serde_json::Value::Null),
        }
    }

    fn command(name: &str) -> String {
        let envelope = format!(
            r#"{{"command":"{name}","params":{{}},"context":{{"moduleId":"nuki","moduleVersion":"1.0.0","propertyId":"790f16ef-4dbb-4295-aa7d-6e0e0ac82ba2","stayId":"a1b2c3d4-e5f6-7890-abcd-ef1234567890","caller":"guest"}}}}"#
        );
        dispatch_command_json(&envelope).expect("dispatched")
    }

    /// Ce qu'une commande renvoie revient à son appelant — sans quoi `getGuestCredential` ne peut
    /// remettre un code à personne, et un `unlock` retombé sur le clavier ne peut pas le dire.
    #[test]
    fn a_command_hands_back_its_result() {
        assert_eq!(command("testUnlock"), r#"{"guest_notice":"Tapez 1234"}"#);
        // Et une commande sans retour répond `null` : au host de n'en rien faire.
        assert_eq!(command("testSubmit"), "null");
    }
}
