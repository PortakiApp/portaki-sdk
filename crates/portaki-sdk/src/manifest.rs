//! Manifest schema types consumed by the CLI when merging macro emissions.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Top-level module manifest (`manifest.json` in the OCI artifact).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModuleManifest {
    /// Manifest schema version.
    pub manifest_version: String,
    /// Module id (slug).
    pub id: String,
    /// Semver from `Cargo.toml`.
    pub version: String,
    /// i18n key for display name.
    pub display_name: String,
    /// i18n key for description.
    pub description: String,
    /// Author metadata.
    pub author: ManifestAuthor,
    /// UI schema versions per shell.
    pub ui_schema: UiSchemaVersions,
    /// Required and optional capabilities.
    pub capabilities: ManifestCapabilities,
    /// Built-in and custom connectors.
    pub connectors: ManifestConnectors,
    /// Entity declarations.
    pub entities: Vec<ManifestEntity>,
    /// Surface declarations grouped by shell.
    pub surfaces: ManifestSurfaces,
    /// Query operations.
    pub queries: Vec<ManifestQuery>,
    /// Command operations.
    pub commands: Vec<ManifestCommand>,
    /// Event declarations.
    pub events: ManifestEvents,
    /// i18n configuration.
    pub i18n: ManifestI18n,
}

/// Module author block.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManifestAuthor {
    /// Author display name.
    pub name: String,
    /// Support URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Support email.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub support_email: Option<String>,
}

/// UI schema version pins.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UiSchemaVersions {
    /// Host SDUI schema version.
    pub host: String,
    /// Guest SDUI schema version.
    pub guest: String,
}

/// Capability requirements in the manifest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ManifestCapabilities {
    /// Required capability ids.
    #[serde(default)]
    pub required: Vec<String>,
    /// Optional capabilities with UX copy keys.
    #[serde(default)]
    pub optional: Vec<ManifestOptionalCapability>,
}

/// Optional capability with purpose/fallback i18n keys.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManifestOptionalCapability {
    /// Capability id.
    pub id: String,
    /// i18n key describing why the capability is needed.
    pub purpose_key: String,
    /// i18n key when the capability is unavailable.
    pub fallback_key: String,
}

/// Connector declarations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ManifestConnectors {
    /// Built-in connector ids.
    #[serde(default)]
    pub builtin: Vec<String>,
    /// Custom connector definitions.
    #[serde(default)]
    pub custom: Vec<Value>,
}

/// Entity schema declaration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ManifestEntity {
    /// Entity name (PascalCase).
    pub name: String,
    /// Schema version for Atlas migrations.
    pub schema_version: u32,
    /// Field metadata (simplified in v1 emissions).
    pub fields: Vec<Value>,
}

/// Surfaces grouped by shell context.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ManifestSurfaces {
    /// Host dashboard surfaces.
    #[serde(default)]
    pub host: Vec<ManifestSurface>,
    /// Guest booklet surfaces.
    #[serde(default)]
    pub guest: Vec<ManifestSurface>,
}

/// Single surface entry in the manifest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManifestSurface {
    /// Surface id (`home.cards`, `main`, …).
    pub id: String,
    /// Rust function symbol invoked by the gateway.
    pub render_fn: String,
    /// Optional i18n key for host navigation labels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name_key: Option<String>,
}

/// Query operation declaration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManifestQuery {
    /// Operation name.
    pub name: String,
    /// Rust function symbol.
    pub r#fn: String,
}

/// Command operation declaration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManifestCommand {
    /// Operation name.
    pub name: String,
    /// Rust function symbol.
    pub r#fn: String,
}

/// Event emit/subscribe declarations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ManifestEvents {
    /// Events emitted by the module.
    #[serde(default)]
    pub emits: Vec<Value>,
    /// Platform events subscribed by the module.
    #[serde(default)]
    pub subscribes: Vec<ManifestEventSubscription>,
}

/// Event subscription entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManifestEventSubscription {
    /// Event type (`core.booking.confirmed`, …).
    pub r#type: String,
    /// Handler function symbol.
    pub handler: String,
}

/// i18n bundle configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManifestI18n {
    /// Default locale (`fr-FR`).
    pub default: String,
    /// Supported locales.
    pub supported: Vec<String>,
}
