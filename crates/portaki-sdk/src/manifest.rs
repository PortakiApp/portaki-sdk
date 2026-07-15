//! Manifest schema types consumed by the Portaki CLI when merging macro emissions.
//!
//! Module authors rarely construct these structs by hand — proc-macros (`portaki_module!`,
//! `surface!`, `entity!`, …) append JSON fragments that the CLI merges into the OCI
//! artifact's `manifest.json`. These types document the **published contract** between
//! the SDK, CLI, and Java gateway.
//!
//! ## Contract
//!
//! - Field names serialize as **camelCase** to match Jackson on the gateway.
//! - `render_fn` / `fn` symbols must match Wasm export names registered via
//!   [`crate::wasm::registry`].
//! - `capabilities.required` must use ids from [`mod@crate::capability`].
//! - `ui_schema` versions pin SDUI primitive schemas — bump only with shell support.
//!
//! ## What modules must not assume
//!
//! - The gateway may reject manifests with unknown capability ids or connector refs.
//! - Optional capabilities listed here are not auto-granted — runtime checks still apply.
//! - Entity `fields` in v1 emissions are simplified metadata; Atlas owns the real DDL.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Top-level module manifest embedded in the OCI artifact as `manifest.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModuleManifest {
    /// Manifest schema version (gateway compatibility gate).
    pub manifest_version: String,
    /// Module slug — matches `portaki_module!(id = "...")`.
    pub id: String,
    /// Crate semver from `Cargo.toml`.
    pub version: String,
    /// i18n key for the marketplace / shell display name.
    pub display_name: String,
    /// i18n key for the module description.
    pub description: String,
    /// Author and support contact block.
    pub author: ManifestAuthor,
    /// SDUI schema version pins per shell.
    pub ui_schema: UiSchemaVersions,
    /// Required and optional platform capabilities.
    pub capabilities: ManifestCapabilities,
    /// Built-in and custom connector declarations.
    pub connectors: ManifestConnectors,
    /// Module-owned entity schemas (Atlas migrations).
    pub entities: Vec<ManifestEntity>,
    /// Surface entries grouped by shell context.
    pub surfaces: ManifestSurfaces,
    /// Read-only query operations exposed to shells.
    pub queries: Vec<ManifestQuery>,
    /// Mutating command operations.
    pub commands: Vec<ManifestCommand>,
    /// Emitted and subscribed domain events.
    pub events: ManifestEvents,
    /// i18n bundle configuration.
    pub i18n: ManifestI18n,
}

/// Author metadata shown in the module marketplace.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManifestAuthor {
    /// Publisher display name.
    pub name: String,
    /// Marketing or documentation URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Support contact email.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub support_email: Option<String>,
}

/// SDUI schema version pins — host and guest shells may diverge.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UiSchemaVersions {
    /// Host dashboard SDUI schema version.
    pub host: String,
    /// Guest booklet SDUI schema version.
    pub guest: String,
}

/// Capability requirements declared at install time.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ManifestCapabilities {
    /// Capability ids that must be granted or installation fails.
    #[serde(default)]
    pub required: Vec<String>,
    /// Optional capabilities with UX copy for purpose and fallback states.
    #[serde(default)]
    pub optional: Vec<ManifestOptionalCapability>,
}

/// Optional capability entry with i18n keys for setup and degraded UX.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManifestOptionalCapability {
    /// Capability id from [`mod@crate::capability`].
    pub id: String,
    /// i18n key explaining why the module wants this capability.
    pub purpose_key: String,
    /// i18n key shown when the capability is unavailable at runtime.
    pub fallback_key: String,
}

/// Connector references — built-in platform connectors and module-defined custom ones.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ManifestConnectors {
    /// Platform connector ids (e.g. `open-weather`).
    #[serde(default)]
    pub builtin: Vec<String>,
    /// Custom connector JSON blobs from `custom_connector!` emissions.
    #[serde(default)]
    pub custom: Vec<Value>,
}

/// Entity declaration merged into Atlas migration planning.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ManifestEntity {
    /// Rust struct name (PascalCase) — maps to repository entity type.
    pub name: String,
    /// Monotonic schema version for forward-compatible migrations.
    pub schema_version: u32,
    /// Simplified field metadata in v1 macro emissions.
    pub fields: Vec<Value>,
}

/// Surface declarations partitioned by shell.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ManifestSurfaces {
    /// Host dashboard surfaces (`render_host_*` aliases).
    #[serde(default)]
    pub host: Vec<ManifestSurface>,
    /// Guest booklet surfaces (`render_guest_*` aliases).
    #[serde(default)]
    pub guest: Vec<ManifestSurface>,
}

/// Single navigable surface entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManifestSurface {
    /// Surface id used in routing (`home.cards`, `main`, …).
    pub id: String,
    /// Wasm symbol invoked by the gateway (macro-generated shim name).
    pub render_fn: String,
    /// Optional i18n key for host navigation labels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name_key: Option<String>,
}

/// Query operation — read-only handler registered in the Wasm registry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManifestQuery {
    /// Operation name referenced by shells and `Action::Command`.
    pub name: String,
    /// Rust function symbol exported through the query shim.
    pub r#fn: String,
}

/// Command operation — mutating handler with JSON params/response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManifestCommand {
    /// Operation name referenced by SDUI actions.
    pub name: String,
    /// Rust function symbol exported through the command shim.
    pub r#fn: String,
}

/// Domain event declarations for publish/subscribe wiring.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ManifestEvents {
    /// Event types emitted via [`crate::host::events::emit`].
    #[serde(default)]
    pub emits: Vec<Value>,
    /// Platform events handled by module functions.
    #[serde(default)]
    pub subscribes: Vec<ManifestEventSubscription>,
}

/// Platform event subscription — gateway dispatches to `handler` symbol.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManifestEventSubscription {
    /// Event type (`core.booking.confirmed`, …).
    pub r#type: String,
    /// Wasm handler symbol from `event_handler!`.
    pub handler: String,
}

/// i18n bundle metadata shipped beside the Wasm binary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManifestI18n {
    /// Default locale when the request has no override.
    pub default: String,
    /// Locales with bundled `.json` translation files.
    pub supported: Vec<String>,
}
