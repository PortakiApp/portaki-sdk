//! Invocation context types passed to surfaces, queries, and commands.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Effective capability grant for the current property.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityGrant {
    /// Capability identifier (e.g. `core.storage`).
    pub id: String,
}

/// Quota usage returned by `host::capabilities::quota`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Quota {
    /// Granted limit (`None` = unlimited).
    pub value: Option<u64>,
    /// Unit hint (`per_day`, `per_month`, …).
    pub unit: Option<String>,
    /// Current usage for the billing period.
    pub used: u64,
}

/// Workspace plan summary attached to the invocation context.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlanInfo {
    /// Public plan family (`free`, `starter`, …).
    pub family: String,
    /// Human-readable plan name.
    pub display_name: String,
}

/// Property-level context injected by the gateway.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PropertyContext {
    /// Property display name.
    pub name: String,
    /// Default locale (`fr-FR`).
    pub locale: String,
    /// Property timezone (`Europe/Paris`).
    pub timezone: String,
    /// Latitude.
    pub lat: f64,
    /// Longitude.
    pub lng: f64,
    /// Single-line address for display.
    pub address: Option<String>,
}

/// Guest identity when rendering guest surfaces.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GuestIdentity {
    /// Opaque guest session id.
    pub session_id: Uuid,
    /// Guest display name if known.
    pub display_name: Option<String>,
    /// Guest locale override.
    pub locale: Option<String>,
}

/// Display preferences from the shell (light/dark, reduced motion, …).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct DisplayPreferences {
    /// Color scheme preference.
    pub scheme: Option<String>,
    /// High contrast requested.
    pub high_contrast: bool,
    /// Reduced motion requested.
    pub reduced_motion: bool,
}

/// Base invocation context for queries, commands, and host surfaces.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Context {
    /// Tenant property id.
    pub property_id: Uuid,
    /// Module identifier from the manifest.
    pub module_id: String,
    /// Semver of the running module artifact.
    pub module_version: String,
    /// Resolved request locale.
    pub locale: String,
    /// Property timezone.
    pub timezone: String,
    /// Effective plan for the workspace.
    pub plan: PlanInfo,
    /// Capabilities available for this invocation.
    pub capabilities: Vec<CapabilityGrant>,
    /// Surface id when rendering a surface (`None` for queries/commands).
    pub surface: Option<String>,
    /// Correlation id for logs and credential access audit.
    pub invocation_id: Uuid,
    /// Shell display preferences.
    pub display: DisplayPreferences,
    /// Guest identity (guest surfaces only).
    pub guest: Option<GuestIdentity>,
    /// Property metadata.
    pub property: PropertyContext,
}

/// Host dashboard surface context.
pub type HostContext = Context;

/// Guest booklet surface context.
pub type GuestContext = Context;

impl Context {
    /// Builds a context with the given capability ids (test / mock helper).
    pub fn with_capabilities(capability_ids: &[&str]) -> Self {
        Context {
            capabilities: capability_ids
                .iter()
                .map(|id| CapabilityGrant {
                    id: (*id).to_string(),
                })
                .collect(),
            property: PropertyContext {
                name: "Villa Azur".to_string(),
                locale: "fr-FR".to_string(),
                timezone: "Europe/Paris".to_string(),
                lat: 43.5513,
                lng: 7.0128,
                address: Some("Cannes, France".to_string()),
            },
            ..Context::default()
        }
    }

    /// Returns whether the effective capability set includes `capability_id`.
    pub fn has_capability(&self, capability_id: &str) -> bool {
        self.capabilities
            .iter()
            .any(|grant| grant.id == capability_id)
    }
}

impl Default for Context {
    fn default() -> Self {
        Self {
            property_id: Uuid::nil(),
            module_id: "test-module".to_string(),
            module_version: "0.0.0".to_string(),
            locale: "fr-FR".to_string(),
            timezone: "Europe/Paris".to_string(),
            plan: PlanInfo {
                family: "free".to_string(),
                display_name: "Free".to_string(),
            },
            capabilities: vec![CapabilityGrant {
                id: "core.storage".to_string(),
            }],
            surface: None,
            invocation_id: Uuid::new_v4(),
            display: DisplayPreferences::default(),
            guest: None,
            property: PropertyContext {
                name: "Test Property".to_string(),
                locale: "fr-FR".to_string(),
                timezone: "Europe/Paris".to_string(),
                lat: 43.55,
                lng: 7.01,
                address: None,
            },
        }
    }
}

/// Clock snapshot for tests.
pub fn fixed_now() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-01-15T12:00:00Z")
        .expect("valid fixture timestamp")
        .with_timezone(&Utc)
}
