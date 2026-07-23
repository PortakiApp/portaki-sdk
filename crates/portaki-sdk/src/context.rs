//! Invocation context types injected by the gateway into every handler.
//!
//! [`Context`] is deserialized from the Wasm request envelope and passed to
//! surface renderers, queries, commands, and event handlers. It is **read-only**
//! from the module's perspective — modules cannot mutate plan, capabilities, or
//! property metadata at runtime.
//!
//! ## Host vs guest surfaces
//!
//! - [`HostContext`] — property dashboard shell (owner/staff).
//! - [`GuestContext`] — guest booklet shell; may include [`GuestIdentity`].
//!
//! Both are type aliases today; the distinction is enforced by which `render_*`
//! symbol the gateway invokes and which manifest surface bucket (`host` / `guest`)
//! declared the surface.
//!
//! ## Capability checks
//!
//! Prefer [`Context::has_capability`] over [`crate::host::capabilities::has`] in
//! render paths — the context snapshot is already paid for and matches what the
//! gateway used to authorize the invocation. Use host probes only when you need
//! a fresh grant after a long-lived cache window.
//!
//! # Examples
//!
//! ```
//! use portaki_sdk::context::{Context, HostContext};
//! use portaki_sdk::capability::core;
//!
//! fn render(ctx: HostContext) -> bool {
//!     ctx.has_capability(core::IMAGES)
//! }
//!
//! let ctx = Context::with_capabilities(&[core::STORAGE, core::IMAGES]);
//! assert!(render(ctx));
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::capability::CapabilityId;
use crate::ids::ModuleId;

/// Single effective capability grant attached to the current invocation.
///
/// The gateway resolves workspace plan + property overrides into this flat list
/// before Wasm execution. Absence from `Context::capabilities` means the
/// capability is not available — do not infer grants from config alone.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityGrant {
    /// Stable capability id (e.g. `core.storage`).
    pub id: String,
}

/// Quota snapshot for a metered capability.
///
/// Populated when the gateway exposes usage for billing-period limits.
/// `value: None` means unlimited; `used` is always present when returned.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Quota {
    /// Granted limit (`None` = unlimited).
    pub value: Option<u64>,
    /// Unit hint for display (`per_day`, `per_month`, …).
    pub unit: Option<String>,
    /// Consumed units in the active billing period.
    pub used: u64,
}

/// Workspace subscription summary for UI copy and entitlement messaging.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlanInfo {
    /// Plan family slug (`free`, `starter`, `pro`, …).
    pub family: String,
    /// Localized marketing name — not an i18n key.
    pub display_name: String,
}

/// Property metadata snapshot for locale, timezone, and map anchoring.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PropertyContext {
    /// Display name shown in guest and host shells.
    pub name: String,
    /// Default locale (`fr-FR`).
    pub locale: String,
    /// IANA timezone (`Europe/Paris`).
    pub timezone: String,
    /// Property latitude (WGS-84).
    pub lat: f64,
    /// Property longitude (WGS-84).
    pub lng: f64,
    /// Single-line formatted address when geocoded.
    pub address: Option<String>,
}

/// Guest session identity on guest booklet surfaces.
///
/// `None` on host surfaces. Never treat `session_id` as authentication proof
/// inside Wasm — the gateway already bound the session before invocation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GuestIdentity {
    /// Opaque guest session identifier.
    pub session_id: Uuid,
    /// Known guest display name, if collected.
    pub display_name: Option<String>,
    /// Guest locale override; falls back to property locale when `None`.
    pub locale: Option<String>,
}

/// Stay window injected on guest booklet invocations.
///
/// Used for timed secret reveal (e.g. access codes). Instants are UTC ISO-8601
/// from the gateway; calendar math for local-time policies uses
/// [`Context::timezone`] / [`PropertyContext::timezone`] (`propertyTimezone`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StayContext {
    /// Stay identifier for the current guest booklet session.
    pub stay_id: Uuid,
    /// Check-in instant (UTC), when the gateway loaded a stay.
    pub checkin_at: Option<DateTime<Utc>>,
    /// Check-out instant (UTC), when the gateway loaded a stay.
    pub checkout_at: Option<DateTime<Utc>>,
}

/// Shell accessibility and theme preferences from the client runtime.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct DisplayPreferences {
    /// Preferred color scheme (`light`, `dark`, `system`).
    pub scheme: Option<String>,
    /// High-contrast mode requested.
    pub high_contrast: bool,
    /// Reduced motion requested — avoid gratuitous animation in SDUI.
    pub reduced_motion: bool,
}

/// Full invocation context for queries, commands, surfaces, and event handlers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Context {
    /// Tenant property identifier.
    pub property_id: Uuid,
    /// Module slug from the manifest (`weather`, `poi`, …).
    pub module_id: ModuleId,
    /// Semver of the Wasm artifact executing this invocation.
    pub module_version: String,
    /// Resolved request locale for i18n and formatting.
    pub locale: String,
    /// Property timezone for calendar math.
    pub timezone: String,
    /// Effective workspace plan at invocation time.
    pub plan: PlanInfo,
    /// Capability grants available for this invocation.
    pub capabilities: Vec<CapabilityGrant>,
    /// Manifest surface id when rendering UI (`None` for queries/commands).
    pub surface: Option<String>,
    /// Correlation id — attach to logs and credential audit trails.
    pub invocation_id: Uuid,
    /// Client display preferences.
    pub display: DisplayPreferences,
    /// Guest identity on guest surfaces only.
    pub guest: Option<GuestIdentity>,
    /// Stay window on guest surfaces (`None` on host / when stay id is absent).
    #[serde(default)]
    pub stay: Option<StayContext>,
    /// Property metadata bundle.
    pub property: PropertyContext,
    /// Surface/query/command input params from the host (route params, overlay args, …).
    #[serde(default)]
    pub input: Value,
}

/// Host dashboard invocation context.
pub type HostContext = Context;

/// Guest booklet invocation context.
pub type GuestContext = Context;

impl Context {
    /// Builds a test context with the given capability ids.
    ///
    /// Fills remaining fields from [`Default`]. Use inside unit tests and
    /// `portaki dev` mocks — not in production handlers.
    pub fn with_capabilities(capability_ids: &[CapabilityId]) -> Self {
        Context {
            capabilities: capability_ids
                .iter()
                .map(|id| CapabilityGrant {
                    id: id.as_str().to_string(),
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

    /// Returns whether `capability_id` is in the effective grant set.
    pub fn has_capability(&self, capability_id: CapabilityId) -> bool {
        let id = capability_id.as_str();
        self.capabilities.iter().any(|grant| grant.id == id)
    }
}

impl Default for Context {
    fn default() -> Self {
        Self {
            property_id: Uuid::nil(),
            module_id: ModuleId::from_static("test-module"),
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
            stay: None,
            property: PropertyContext {
                name: "Test Property".to_string(),
                locale: "fr-FR".to_string(),
                timezone: "Europe/Paris".to_string(),
                lat: 43.55,
                lng: 7.01,
                address: None,
            },
            input: Value::Null,
        }
    }
}

/// Fixed UTC timestamp for deterministic tests (`2026-01-15T12:00:00Z`).
pub fn fixed_now() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-01-15T12:00:00Z")
        .expect("valid fixture timestamp")
        .with_timezone(&Utc)
}
