//! Merges `OUT_DIR/portaki-emissions/*.json` into `manifest.json`.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use portaki_sdk::manifest::{
    ManifestAuthor, ManifestCapabilities, ManifestCommand, ManifestConnectors, ManifestEntity,
    ManifestEventSubscription, ManifestEvents, ManifestI18n, ManifestOptionalCapability,
    ManifestQuery, ManifestSurface, ManifestSurfaces, ModuleManifest, UiSchemaVersions,
};
use serde::Deserialize;
use serde_json::Value;
use walkdir::WalkDir;

/// One emission file produced by proc-macros during `cargo build`.
#[derive(Debug, Clone, Deserialize)]
pub struct EmissionFile {
    /// Emission kind (`module`, `surface`, …).
    pub kind: String,
    #[serde(flatten)]
    pub data: Value,
}

/// Collects emission JSON files from a directory tree.
pub fn collect_emissions(root: &Path) -> Result<Vec<EmissionFile>> {
    let mut files = Vec::new();
    if !root.exists() {
        return Ok(files);
    }

    for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let text = fs::read_to_string(path)
            .with_context(|| format!("read emission {}", path.display()))?;
        let parsed: EmissionFile =
            serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
        files.push(parsed);
    }

    Ok(files)
}

/// Generates a [`ModuleManifest`] from emission files and crate metadata.
pub fn generate_manifest(
    emissions: &[EmissionFile],
    default_locale: &str,
    supported_locales: &[String],
) -> Result<ModuleManifest> {
    let module = emissions
        .iter()
        .find(|e| e.kind == "module")
        .context("missing module emission — add portaki_module!(...) to lib.rs")?;

    let id = module.data["id"]
        .as_str()
        .context("module emission missing id")?
        .to_string();
    let version = module.data["version"]
        .as_str()
        .unwrap_or("0.1.0")
        .to_string();
    let display_name = module.data["displayName"]
        .as_str()
        .unwrap_or("module.displayName")
        .to_string();
    let description = module.data["description"]
        .as_str()
        .unwrap_or("module.description")
        .to_string();
    let author_name = module.data["author"]["name"]
        .as_str()
        .unwrap_or("Syntax Labs")
        .to_string();

    let mut required_caps = Vec::new();
    let mut optional_caps = Vec::new();
    let mut provided_caps = Vec::new();
    let mut builtin_connectors = Vec::new();
    let mut custom_connectors: Vec<Value> = Vec::new();

    for emission in emissions {
        match emission.kind.as_str() {
            "capability" => {
                let id = emission.data["id"].as_str().unwrap_or_default().to_string();
                if emission.data["provided"].as_bool().unwrap_or(false) {
                    if !id.is_empty() {
                        provided_caps.push(id);
                    }
                } else if emission.data["optional"].as_bool().unwrap_or(false) {
                    optional_caps.push(ManifestOptionalCapability {
                        id,
                        purpose_key: emission.data["purposeKey"]
                            .as_str()
                            .unwrap_or("capability.purpose")
                            .to_string(),
                        fallback_key: emission.data["fallbackKey"]
                            .as_str()
                            .unwrap_or("capability.fallback")
                            .to_string(),
                    });
                } else if !id.is_empty() {
                    required_caps.push(id);
                }
            }
            "connector_builtin" => {
                if let Some(id) = emission.data["id"].as_str() {
                    builtin_connectors.push(id.to_string());
                }
            }
            "connector_custom" => {
                custom_connectors.push(serde_json::json!({
                    "id": emission.data["id"],
                    "displayNameKey": emission.data["displayNameKey"],
                    "baseUrl": emission.data["baseUrl"],
                    "credentialProviderId": emission.data["credentialProviderId"],
                    "operations": []
                }));
            }
            _ => {}
        }
    }

    for emission in emissions {
        if emission.kind != "connector_op" {
            continue;
        }
        let operation = serde_json::json!({
            "id": emission.data["fn"],
            "method": emission.data["method"],
            "path": emission.data["path"],
        });
        if let Some(connector) = custom_connectors.last_mut() {
            if let Some(ops) = connector
                .get_mut("operations")
                .and_then(Value::as_array_mut)
            {
                ops.push(operation);
            }
        }
    }

    let mut host_surfaces = Vec::new();
    let mut guest_surfaces = Vec::new();
    for emission in emissions {
        if emission.kind != "surface" {
            continue;
        }
        let surface = ManifestSurface {
            id: emission.data["id"].as_str().unwrap_or_default().to_string(),
            render_fn: emission.data["renderFn"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            display_name_key: emission.data["displayNameKey"].as_str().map(str::to_string),
        };
        match emission.data["context"].as_str().unwrap_or("guest") {
            "host" => host_surfaces.push(surface),
            _ => guest_surfaces.push(surface),
        }
    }

    let mut queries = Vec::new();
    let mut commands = Vec::new();
    let mut subscribes = Vec::new();
    let mut entities = Vec::new();

    for emission in emissions {
        match emission.kind.as_str() {
            "query" => queries.push(ManifestQuery {
                name: emission.data["name"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                r#fn: emission.data["fn"].as_str().unwrap_or_default().to_string(),
            }),
            "command" => commands.push(ManifestCommand {
                name: emission.data["name"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                r#fn: emission.data["fn"].as_str().unwrap_or_default().to_string(),
            }),
            "event_handler" => subscribes.push(ManifestEventSubscription {
                r#type: emission.data["type"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                handler: emission.data["handler"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            }),
            "entity" => entities.push(ManifestEntity {
                name: emission.data["name"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                schema_version: emission.data["schemaVersion"].as_u64().unwrap_or(1) as u32,
                fields: emission.data["fields"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default(),
            }),
            _ => {}
        }
    }

    Ok(ModuleManifest {
        manifest_version: "1".to_string(),
        id,
        version,
        display_name,
        description,
        author: ManifestAuthor {
            name: author_name,
            url: Some("https://syntax-labs.fr".to_string()),
            support_email: Some("support@syntax-labs.fr".to_string()),
        },
        ui_schema: UiSchemaVersions {
            host: "1".to_string(),
            guest: "1".to_string(),
        },
        capabilities: ManifestCapabilities {
            required: required_caps,
            optional: optional_caps,
            provided: provided_caps,
        },
        connectors: ManifestConnectors {
            builtin: builtin_connectors,
            custom: custom_connectors,
        },
        entities,
        surfaces: ManifestSurfaces {
            host: host_surfaces,
            guest: guest_surfaces,
        },
        queries,
        commands,
        events: ManifestEvents {
            emits: Vec::new(),
            subscribes,
        },
        i18n: ManifestI18n {
            default: default_locale.to_string(),
            supported: supported_locales.to_vec(),
        },
    })
}

/// Writes `manifest.json` to `dest`.
pub fn write_manifest(manifest: &ModuleManifest, dest: &Path) -> Result<()> {
    let json = serde_json::to_string_pretty(manifest)?;
    fs::write(dest, json).with_context(|| format!("write {}", dest.display()))?;
    Ok(())
}

/// Finds the most recent `portaki-emissions` directory under `target/`.
pub fn find_emissions_dir(module_root: &Path) -> Option<PathBuf> {
    let target = module_root.join("target");
    if !target.exists() {
        return None;
    }

    let mut candidates = Vec::new();
    for entry in WalkDir::new(&target).into_iter().filter_map(Result::ok) {
        if entry.file_name() == "portaki-emissions" {
            candidates.push(entry.path().to_path_buf());
        }
    }

    candidates.sort();
    candidates.pop()
}
