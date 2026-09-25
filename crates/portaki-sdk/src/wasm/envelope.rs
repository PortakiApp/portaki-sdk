//! JSON envelope passed across the Extism boundary (`portaki_query` / `portaki_command`).

use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use chrono::{DateTime, NaiveTime, Utc};

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
    /// Who calls: `host` or `guest`. Absent (older runtime) or unknown, a stay means a guest.
    #[serde(default)]
    pub caller: Option<String>,
    /// Stay the invocation is about, for a guest or a host caller.
    #[serde(rename = "stayId", default)]
    pub stay_id: Option<Uuid>,
    /// Stay check-in instant (UTC ISO-8601) for guest reveal policies.
    #[serde(rename = "checkinAt", default)]
    pub checkin_at: Option<String>,
    /// Stay check-out instant (UTC ISO-8601) for guest reveal policies.
    #[serde(rename = "checkoutAt", default)]
    pub checkout_at: Option<String>,
    /// Platform the stay was booked on (lowercased channel name). Optional for
    /// backward compatibility with older orchestrators that do not send it.
    #[serde(rename = "bookingChannel", default)]
    pub booking_channel: Option<String>,
    /// Number of guests on the stay.
    #[serde(rename = "guestPartySize", default)]
    pub guest_party_size: Option<u32>,
    /// Announced arrival time, property-local (`HH:mm` or `HH:mm:ss`).
    #[serde(rename = "arrivalTimeEstimated", default)]
    pub arrival_time_estimated: Option<String>,
    /// Language the guest communicates in, as the stay stores it.
    #[serde(rename = "guestLocale", default)]
    pub guest_locale: Option<String>,
    /// Guest email — present only when the manifest declares `stay:guest_contact:read`.
    #[serde(rename = "guestEmail", default)]
    pub guest_email: Option<String>,
    /// Guest phone — present only when the manifest declares `stay:guest_contact:read`.
    #[serde(rename = "guestPhone", default)]
    pub guest_phone: Option<String>,
    /// Property IANA timezone (`Europe/Paris`) — preferred over [`Self::timezone`].
    #[serde(rename = "propertyTimezone", default)]
    pub property_timezone: Option<String>,
    /// Effective capability ids (orchestrator passes as scopes).
    #[serde(default)]
    pub scopes: Vec<String>,
    /// Module / property context JSON blob (orchestrator serializes `propertyContext` here).
    #[serde(rename = "configJson", default)]
    pub config_json: String,
    /// The install's configuration, decrypted (`property_module.config_json`); `{}` when none.
    /// Distinct from [`Self::config_json`], the property context.
    #[serde(rename = "moduleConfig", default)]
    pub module_config: Value,
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
            booking_channel: ctx
                .booking_channel
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| value.to_ascii_lowercase()),
            party_size: ctx.guest_party_size.filter(|size| *size > 0),
            arrival_time_estimated: parse_local_time_opt(ctx.arrival_time_estimated.as_deref()),
            guest_locale: non_blank(ctx.guest_locale.as_deref()),
            // Le runtime ne sérialise ces deux champs que pour un module qui déclare
            // `stay:guest_contact:read` ; le SDK ne refait pas ce contrôle, il n'a pas le manifeste.
            guest_email: non_blank(ctx.guest_email.as_deref()),
            guest_phone: non_blank(ctx.guest_phone.as_deref()),
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
            guest: ctx
                .stay_id
                .filter(|_| ctx.caller.as_deref() != Some("host"))
                .map(|session_id| GuestIdentity {
                    session_id,
                    display_name: None,
                    locale: None,
                }),
            stay,
            property,
            input: self.params.clone(),
            module_config: match &ctx.module_config {
                // Un runtime antérieur n'envoie rien : `{}`, comme une install sans config.
                Value::Null => Value::Object(serde_json::Map::new()),
                config => config.clone(),
            },
        })
    }
}

/// `HH:mm` comme le runtime l'envoie, `HH:mm:ss` par tolérance ; illisible vaut absent.
fn parse_local_time_opt(raw: Option<&str>) -> Option<NaiveTime> {
    let value = raw?.trim();
    NaiveTime::parse_from_str(value, "%H:%M")
        .or_else(|_| NaiveTime::parse_from_str(value, "%H:%M:%S"))
        .ok()
}

fn non_blank(raw: Option<&str>) -> Option<String> {
    raw.map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
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
        assert_eq!(ctx.module_config, serde_json::json!({}));
    }

    /// `moduleConfig` is the install's config, next to `configJson` which stays the property.
    #[test]
    fn reads_module_config_apart_from_the_property_context() {
        let raw = r#"{
            "query": "getCurrent",
            "context": {
                "moduleId": "wifi",
                "moduleVersion": "1.0.0",
                "propertyId": "790f16ef-4dbb-4295-aa7d-6e0e0ac82ba2",
                "configJson": "{\"name\":\"Vayoux\"}",
                "moduleConfig": { "ssid": "Vayoux-5G", "password": "s3cret" }
            }
        }"#;
        let envelope: WasmRequestEnvelope = serde_json::from_str(raw).expect("parse");
        let ctx = envelope.to_context("getCurrent").expect("context");
        assert_eq!(ctx.property.name, "Vayoux");
        assert_eq!(ctx.module_config["ssid"], "Vayoux-5G");
        assert_eq!(ctx.module_config["password"], "s3cret");
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
        // Un runtime antérieur n'envoie aucun des champs enrichis : tous restent absents.
        assert_eq!(stay.party_size, None);
        assert_eq!(stay.arrival_time_estimated, None);
        assert_eq!(stay.guest_locale, None);
        assert_eq!(stay.guest_email, None);
        assert_eq!(stay.guest_phone, None);
    }

    #[test]
    fn reads_stay_details_and_granted_guest_contact() {
        let raw = r#"{
            "query": "render_guest_home_cards",
            "params": {},
            "context": {
                "moduleId": "checkin",
                "moduleVersion": "1.0.0",
                "propertyId": "790f16ef-4dbb-4295-aa7d-6e0e0ac82ba2",
                "stayId": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
                "bookingChannel": "AIRBNB",
                "guestPartySize": 4,
                "arrivalTimeEstimated": "17:30",
                "guestLocale": "de",
                "guestEmail": "anna@example.com",
                "guestPhone": " "
            }
        }"#;
        let envelope: WasmRequestEnvelope = serde_json::from_str(raw).expect("parse");
        let stay = envelope
            .to_context("render_guest_home_cards")
            .expect("context")
            .stay
            .expect("stay");

        assert_eq!(stay.booking_channel.as_deref(), Some("airbnb"));
        assert_eq!(stay.party_size, Some(4));
        assert_eq!(
            stay.arrival_time_estimated,
            chrono::NaiveTime::from_hms_opt(17, 30, 0)
        );
        assert_eq!(stay.guest_locale.as_deref(), Some("de"));
        assert_eq!(stay.guest_email.as_deref(), Some("anna@example.com"));
        assert_eq!(
            stay.guest_phone, None,
            "a blank value is not a phone number"
        );
    }

    fn context_for(caller: &str) -> crate::Context {
        let raw = format!(
            r#"{{
                "command": "resolve",
                "context": {{
                    "moduleId": "issue-report",
                    "moduleVersion": "1.0.0",
                    "propertyId": "790f16ef-4dbb-4295-aa7d-6e0e0ac82ba2",
                    "stayId": "a1b2c3d4-e5f6-7890-abcd-ef1234567890"
                    {caller}
                }}
            }}"#
        );
        let envelope: WasmRequestEnvelope = serde_json::from_str(&raw).expect("parse");
        envelope.to_context("resolve").expect("context")
    }

    /// Le séjour reste là pour tout appelant ; seul un hôte déclaré n'est pas un voyageur.
    #[test]
    fn guest_follows_the_caller_and_the_stay_stays() {
        for (caller, is_guest) in [
            (r#","caller": "host""#, false),
            (r#","caller": "guest""#, true),
            ("", true),
            (r#","caller": "robot""#, true),
        ] {
            let ctx = context_for(caller);
            assert_eq!(ctx.guest.is_some(), is_guest, "{caller}");
            assert!(ctx.stay.is_some(), "{caller}");
        }
    }

    /// Les valeurs nulles — ce qu'un module non déclarant reçoit pour le contact — et une heure
    /// illisible se lisent comme absentes, sans faire échouer l'invocation.
    #[test]
    fn null_contact_and_unreadable_arrival_time_are_absent() {
        let raw = r#"{
            "command": "submit",
            "context": {
                "moduleId": "checkin",
                "moduleVersion": "1.0.0",
                "propertyId": "790f16ef-4dbb-4295-aa7d-6e0e0ac82ba2",
                "stayId": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
                "guestPartySize": 0,
                "arrivalTimeEstimated": "vers 18h",
                "guestEmail": null,
                "guestPhone": null
            }
        }"#;
        let envelope: WasmRequestEnvelope = serde_json::from_str(raw).expect("parse");
        let stay = envelope
            .to_context("submit")
            .expect("context")
            .stay
            .expect("stay");

        assert_eq!(stay.party_size, None);
        assert_eq!(stay.arrival_time_estimated, None);
        assert_eq!(stay.guest_email, None);
        assert_eq!(stay.guest_phone, None);
    }

    /// `StayContext` round-trips, and a payload without the new fields still deserializes.
    #[test]
    fn stay_context_serde_keeps_optional_fields_optional() {
        let legacy: crate::StayContext = serde_json::from_str(
            r#"{"stay_id":"a1b2c3d4-e5f6-7890-abcd-ef1234567890","checkin_at":null,"checkout_at":null}"#,
        )
        .expect("legacy stay");
        assert_eq!(legacy.guest_email, None);
        assert_eq!(legacy.party_size, None);

        let full = crate::StayContext {
            party_size: Some(2),
            arrival_time_estimated: chrono::NaiveTime::from_hms_opt(9, 15, 0),
            guest_locale: Some("fr".to_string()),
            guest_email: Some("marie@example.com".to_string()),
            guest_phone: Some("+33600000000".to_string()),
            ..legacy
        };
        let wire = serde_json::to_string(&full).expect("serialize");
        let back: crate::StayContext = serde_json::from_str(&wire).expect("deserialize");
        assert_eq!(back, full);
    }
}
