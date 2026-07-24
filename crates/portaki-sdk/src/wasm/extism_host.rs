//! Production Wasm host backend — calls Java `portaki_host_dispatch` via Extism.

use std::sync::Arc;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use extism_pdk::*;
use serde_json::{json, Value};

use crate::context::Context;
use crate::error::{PortakiError, Result};
use crate::host::runtime::{context_or_load, HostBackend};

#[host_fn]
extern "ExtismHost" {
    fn portaki_host_dispatch(input: String) -> String;
}

/// Host backend backed by the Java gateway (`portaki_host_dispatch`).
pub struct ExtismHostBackend;

impl ExtismHostBackend {
    fn dispatch_value(&self, op: &str, args: Value) -> Result<Value> {
        let request = json!({ "op": op, "args": args });
        let request_json =
            serde_json::to_string(&request).map_err(|e| PortakiError::Host(e.to_string()))?;
        let response_json = unsafe { portaki_host_dispatch(request_json) }
            .map_err(|e| PortakiError::Host(format!("portaki_host_dispatch: {e}")))?;
        parse_response(&response_json)
    }
}

impl HostBackend for ExtismHostBackend {
    fn context(&self) -> Result<Context> {
        context_or_load()
    }

    fn has_capability(&self, id: &str) -> Result<bool> {
        if let Ok(ctx) = context_or_load() {
            if ctx.capabilities.iter().any(|grant| grant.id == id) {
                return Ok(true);
            }
        }
        let result = self.dispatch_value("capabilities.has", json!({ "id": id }))?;
        Ok(result
            .get("granted")
            .and_then(Value::as_bool)
            .unwrap_or(false))
    }

    fn kv_get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let result = self.dispatch_value("kv.get", json!({ "key": key }))?;
        let Some(encoded) = result.get("value").and_then(Value::as_str) else {
            return Ok(None);
        };
        let bytes = BASE64
            .decode(encoded)
            .map_err(|e| PortakiError::Host(format!("kv_get_base64_decode: {e}")))?;
        Ok(Some(bytes))
    }

    fn kv_set(&self, key: &str, value: &[u8], ttl_seconds: Option<u32>) -> Result<()> {
        let mut args = json!({
            "key": key,
            "value": BASE64.encode(value),
        });
        if let Some(ttl) = ttl_seconds {
            args["ttlSeconds"] = json!(ttl);
        }
        self.dispatch_value("kv.set", args)?;
        Ok(())
    }

    fn kv_delete(&self, key: &str) -> Result<()> {
        self.dispatch_value("kv.delete", json!({ "key": key }))?;
        Ok(())
    }

    fn kv_list(&self, prefix: &str) -> Result<Vec<String>> {
        let result = self.dispatch_value("kv.list", json!({ "prefix": prefix }))?;
        let keys = result
            .get("keys")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        Ok(keys)
    }

    fn i18n_translate(&self, key: &str, vars_json: &str) -> Result<String> {
        let result = self.dispatch_value(
            "i18n.translate",
            json!({ "key": key, "varsJson": vars_json }),
        )?;
        Ok(result
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or(key)
            .to_string())
    }

    fn log(&self, level: &str, message: &str, fields_json: &str) -> Result<()> {
        let _ = self.dispatch_value(
            "log",
            json!({ "level": level, "message": message, "fieldsJson": fields_json }),
        );
        Ok(())
    }

    fn connector_call(
        &self,
        connector_id: &str,
        operation: &str,
        args_json: &str,
    ) -> Result<String> {
        let result = self.dispatch_value(
            "connector.call",
            json!({
                "connectorId": connector_id,
                "operation": operation,
                "argsJson": args_json,
            }),
        )?;
        result
            .get("json")
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| PortakiError::Host("connector_call_missing_json".into()))
    }

    fn emit_event(&self, event_type: &str, payload_json: &str) -> Result<()> {
        self.dispatch_value(
            "events.emit",
            json!({
                "eventType": event_type,
                "payloadJson": payload_json,
            }),
        )?;
        Ok(())
    }

    fn email_send(&self, payload_json: &str) -> Result<()> {
        self.dispatch_value(
            "email.send",
            json!({
                "payloadJson": payload_json,
            }),
        )?;
        Ok(())
    }

    fn time_now_iso(&self) -> Result<String> {
        let result = self.dispatch_value("time.now", json!({}))?;
        result
            .get("iso")
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| PortakiError::Host("time_now_missing_iso".into()))
    }

    fn repo_find(&self, entity: &str, query_json: &str) -> Result<String> {
        let result = self.dispatch_value(
            "repo.find",
            json!({ "entity": entity, "queryJson": query_json }),
        )?;
        serde_json::to_string(&result).map_err(|e| PortakiError::Host(e.to_string()))
    }

    fn repo_create(&self, entity: &str, entity_json: &str) -> Result<String> {
        let result = self.dispatch_value(
            "repo.create",
            json!({ "entity": entity, "entityJson": entity_json }),
        )?;
        serde_json::to_string(&result).map_err(|e| PortakiError::Host(e.to_string()))
    }

    fn repo_delete(&self, entity: &str, id: &str) -> Result<bool> {
        let result = self.dispatch_value("repo.delete", json!({ "entity": entity, "id": id }))?;
        Ok(result
            .get("deleted")
            .and_then(Value::as_bool)
            .unwrap_or(false))
    }

    fn module_status(&self) -> Result<crate::host::module::ModuleStatus> {
        let result = self.dispatch_value("module.status", json!({}))?;
        serde_json::from_value(result)
            .map_err(|e| PortakiError::Host(format!("module_status_parse_failed: {e}")))
    }

    fn module_list_by_capability(
        &self,
        capability_id: &str,
    ) -> Result<Vec<crate::host::module::ModulePeer>> {
        let result = self.dispatch_value(
            "module.listByCapability",
            json!({ "capabilityId": capability_id }),
        )?;
        let modules = result
            .get("modules")
            .cloned()
            .unwrap_or(Value::Array(Vec::new()));
        serde_json::from_value(modules)
            .map_err(|e| PortakiError::Host(format!("module_list_by_capability_parse_failed: {e}")))
    }
}

fn parse_response(response_json: &str) -> Result<Value> {
    let root: Value = serde_json::from_str(response_json)
        .map_err(|e| PortakiError::Host(format!("host_dispatch_parse_failed: {e}")))?;
    if root.get("ok").and_then(Value::as_bool) != Some(true) {
        let code = root
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("host_dispatch_error");
        let message = root.get("message").and_then(Value::as_str).unwrap_or("");
        return Err(PortakiError::Host(format!("{code}: {message}")));
    }
    Ok(root
        .get("result")
        .cloned()
        .unwrap_or(Value::Object(Default::default())))
}

#[cfg(test)]
mod tests {
    use super::parse_response;

    #[test]
    fn parses_ok_response() {
        let value = parse_response(r#"{"ok":true,"result":{"value":"YQ=="}}"#).expect("parse");
        assert_eq!(value["value"], "YQ==");
    }

    #[test]
    fn rejects_error_response() {
        let err = parse_response(r#"{"ok":false,"error":"x","message":"y"}"#).unwrap_err();
        assert!(err.to_string().contains('x'));
    }
}
