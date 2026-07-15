//! JSON envelope passed across the Extism boundary (`portaki_query` / `portaki_command`).

use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use crate::context::{
    CapabilityGrant, Context, DisplayPreferences, GuestIdentity, PlanInfo, PropertyContext,
};
use crate::error::{PortakiError, Result};

/// Host → module request body (matches the Portaki Wasm invocation payload on the host).
#[derive(Debug, Deserialize)]
pub struct WasmRequestEnvelope {
    /// Query or surface handler name.
    pub query: Option<String>,
    /// Command handler name.
    pub command: Option<String>,
    /// Operation parameters.
    #[serde(default)]
    pub params: Value,
    /// Invocation context.
    pub context: WasmContextEnvelope,
}

/// Context subset serialized by the Java runtime (Jackson camelCase).
#[derive(Debug, Deserialize)]
pub struct WasmContextEnvelope {
    /// Module id (e.g. `weather`).
    #[serde(rename = "moduleId")]
    pub module_id: String,
    /// Pinned module version.
    #[serde(rename = "moduleVersion")]
    pub module_version: String,
    /// Workspace id.
    #[serde(rename = "workspaceId", default)]
    pub workspace_id: Option<Uuid>,
    /// Property id.
    #[serde(rename = "propertyId", default)]
    pub property_id: Option<Uuid>,
    /// Stay id for guest invocations.
    #[serde(rename = "stayId", default)]
    pub stay_id: Option<Uuid>,
    /// Effective capability ids (orchestrator passes as scopes).
    #[serde(default)]
    pub scopes: Vec<String>,
    /// Module config JSON blob.
    #[serde(rename = "configJson", default)]
    pub config_json: String,
    /// Request locale (`fr-FR`).
    #[serde(default)]
    pub locale: Option<String>,
    /// Property timezone (`Europe/Paris`).
    #[serde(default)]
    pub timezone: Option<String>,
}

impl WasmRequestEnvelope {
    /// Resolves the operation name (query or command).
    pub fn operation_name(&self) -> Result<&str> {
        if let Some(query) = self.query.as_deref() {
            if !query.is_empty() {
                return Ok(query);
            }
        }
        if let Some(command) = self.command.as_deref() {
            if !command.is_empty() {
                return Ok(command);
            }
        }
        Err(PortakiError::Host(
            "wasm_envelope_missing_operation".to_string(),
        ))
    }

    /// Builds a module [`Context`] from the envelope (capabilities from `scopes`).
    pub fn to_context(&self, operation: &str) -> Result<Context> {
        let ctx = &self.context;
        let property_id = ctx
            .property_id
            .ok_or_else(|| PortakiError::Host("wasm_context_missing_property_id".to_string()))?;
        let capabilities = ctx
            .scopes
            .iter()
            .map(|id| CapabilityGrant { id: id.clone() })
            .collect();
        let locale = ctx.locale.clone().unwrap_or_else(|| "fr-FR".to_string());
        let timezone = ctx
            .timezone
            .clone()
            .unwrap_or_else(|| "Europe/Paris".to_string());
        let property_locale = locale.clone();
        Ok(Context {
            property_id,
            module_id: ctx.module_id.clone(),
            module_version: ctx.module_version.clone(),
            locale,
            timezone: timezone.clone(),
            plan: PlanInfo {
                family: "starter".to_string(),
                display_name: "Starter".to_string(),
            },
            capabilities,
            surface: Some(operation.to_string()),
            invocation_id: Uuid::new_v4(),
            display: DisplayPreferences::default(),
            guest: ctx.stay_id.map(|session_id| GuestIdentity {
                session_id,
                display_name: None,
                locale: None,
            }),
            property: PropertyContext {
                name: "Property".to_string(),
                locale: property_locale,
                timezone,
                lat: 48.8566,
                lng: 2.3522,
                address: None,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::WasmRequestEnvelope;

    #[test]
    fn parses_java_camel_case_envelope() {
        let raw = r#"{
            "query": "getCurrent",
            "params": {},
            "context": {
                "moduleId": "weather",
                "moduleVersion": "0.3.0",
                "propertyId": "790f16ef-4dbb-4295-aa7d-6e0e0ac82ba2",
                "scopes": ["external.open-weather.pool"]
            }
        }"#;
        let envelope: WasmRequestEnvelope = serde_json::from_str(raw).expect("parse");
        assert_eq!(envelope.operation_name().unwrap(), "getCurrent");
        let ctx = envelope.to_context("getCurrent").expect("context");
        assert_eq!(ctx.module_id, "weather");
        assert_eq!(ctx.capabilities.len(), 1);
    }
}
