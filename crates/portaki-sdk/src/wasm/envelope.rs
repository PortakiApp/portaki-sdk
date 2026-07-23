//! JSON envelope passed across the Extism boundary (`portaki_query` / `portaki_command`).

use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use chrono::{DateTime, Utc};

use crate::context::{
    CapabilityGrant, Context, DisplayPreferences, GuestIdentity, PlanInfo, PropertyContext,
    StayContext,
};
use crate::error::{PortakiError, Result};
use crate::ids::ModuleId;

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
    /// Stay check-in instant (UTC ISO-8601) for guest reveal policies.
    #[serde(rename = "checkinAt", default)]
    pub checkin_at: Option<String>,
    /// Stay check-out instant (UTC ISO-8601) for guest reveal policies.
    #[serde(rename = "checkoutAt", default)]
    pub checkout_at: Option<String>,
    /// Property IANA timezone (`Europe/Paris`) — preferred over [`Self::timezone`].
    #[serde(rename = "propertyTimezone", default)]
    pub property_timezone: Option<String>,
    /// Effective capability ids (orchestrator passes as scopes).
    #[serde(default)]
    pub scopes: Vec<String>,
    /// Module / property context JSON blob (orchestrator serializes `propertyContext` here).
    #[serde(rename = "configJson", default)]
    pub config_json: String,
    /// Request locale (`fr-FR`).
    #[serde(default)]
    pub locale: Option<String>,
    /// Property timezone (`Europe/Paris`) — legacy alias; prefer `propertyTimezone`.
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
            .property_timezone
            .clone()
            .or_else(|| ctx.timezone.clone())
            .unwrap_or_else(|| "Europe/Paris".to_string());
        let property_locale = locale.clone();
        let property =
            property_from_config_json(&ctx.config_json, property_locale, timezone.clone());
        let stay = ctx.stay_id.map(|stay_id| StayContext {
            stay_id,
            checkin_at: parse_instant_opt(ctx.checkin_at.as_deref()),
            checkout_at: parse_instant_opt(ctx.checkout_at.as_deref()),
        });
        Ok(Context {
            property_id,
            module_id: ModuleId::new(ctx.module_id.clone()),
            module_version: ctx.module_version.clone(),
            locale,
            timezone,
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
            stay,
            property,
            input: self.params.clone(),
        })
    }
}

fn parse_instant_opt(raw: Option<&str>) -> Option<DateTime<Utc>> {
    let value = raw?.trim();
    if value.is_empty() {
        return None;
    }
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

fn property_from_config_json(
    config_json: &str,
    locale: String,
    timezone: String,
) -> PropertyContext {
    let parsed: Value = if config_json.is_empty() {
        Value::Null
    } else {
        serde_json::from_str(config_json).unwrap_or(Value::Null)
    };
    let name = parsed
        .get("name")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .unwrap_or("Property")
        .to_string();
    let address = parsed
        .get("address")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let lat = parsed.get("lat").and_then(Value::as_f64).unwrap_or(48.8566);
    let lng = parsed.get("lng").and_then(Value::as_f64).unwrap_or(2.3522);
    PropertyContext {
        name,
        locale,
        timezone,
        lat,
        lng,
        address,
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
        assert!((ctx.property.lat - 48.8566).abs() < f64::EPSILON);
    }

    #[test]
    fn reads_property_coordinates_from_config_json() {
        let raw = r#"{
            "query": "getCurrent",
            "params": {},
            "context": {
                "moduleId": "weather",
                "moduleVersion": "0.3.7",
                "propertyId": "790f16ef-4dbb-4295-aa7d-6e0e0ac82ba2",
                "scopes": ["external.open-weather.byok"],
                "configJson": "{\"name\":\"Vayoux\",\"lat\":45.764,\"lng\":4.8357,\"address\":\"Lyon\"}"
            }
        }"#;
        let envelope: WasmRequestEnvelope = serde_json::from_str(raw).expect("parse");
        let ctx = envelope.to_context("getCurrent").expect("context");
        assert_eq!(ctx.property.name, "Vayoux");
        assert_eq!(ctx.property.address.as_deref(), Some("Lyon"));
        assert!((ctx.property.lat - 45.764).abs() < f64::EPSILON);
        assert!((ctx.property.lng - 4.8357).abs() < f64::EPSILON);
    }

    #[test]
    fn reads_guest_stay_window_and_property_timezone() {
        let raw = r#"{
            "query": "render_guest_home_cards",
            "params": {},
            "context": {
                "moduleId": "example-module",
                "moduleVersion": "0.2.0",
                "propertyId": "790f16ef-4dbb-4295-aa7d-6e0e0ac82ba2",
                "stayId": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
                "checkinAt": "2026-07-20T14:00:00Z",
                "checkoutAt": "2026-07-25T10:00:00Z",
                "propertyTimezone": "Europe/Paris",
                "scopes": ["core.storage"]
            }
        }"#;
        let envelope: WasmRequestEnvelope = serde_json::from_str(raw).expect("parse");
        let ctx = envelope
            .to_context("render_guest_home_cards")
            .expect("context");
        let stay = ctx.stay.expect("stay");
        assert_eq!(
            stay.stay_id.to_string(),
            "a1b2c3d4-e5f6-7890-abcd-ef1234567890"
        );
        assert_eq!(
            stay.checkin_at
                .expect("checkin")
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            "2026-07-20T14:00:00Z"
        );
        assert_eq!(
            stay.checkout_at
                .expect("checkout")
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            "2026-07-25T10:00:00Z"
        );
        assert_eq!(ctx.timezone, "Europe/Paris");
        assert_eq!(ctx.property.timezone, "Europe/Paris");
    }
}
