//! Merges `OUT_DIR/portaki-emissions/*.json` into `manifest.json`.

use std::fs;
use std::path::{Path, PathBuf};

/// La version de manifeste que cette version du CLI produit.
///
/// Nommée plutôt que répétée en littéral : `ci check` compare le manifeste d'un module à cette
/// référence, et deux littéraux qui doivent rester égaux finissent toujours par diverger.
pub const MANIFEST_VERSION: &str = "1";

/// La version de schéma SDUI que cette version du CLI produit, pour les deux coquilles.
pub const SDUI_SCHEMA_VERSION: &str = "1";

use anyhow::{Context, Result};
use portaki_sdk::capability::CapabilityId;
use portaki_sdk::manifest::{
    ManifestAuthor, ManifestCapabilities, ManifestCommand, ManifestConnectors, ManifestEntity,
    ManifestEventSubscription, ManifestEvents, ManifestI18n, ManifestOptionalCapability,
    ManifestQuery, ManifestSurface, ManifestSurfaces, ModuleManifest, UiSchemaVersions,
};
use serde::Deserialize;
use serde_json::Value;
use std::str::FromStr;
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
                let id_raw = emission.data["id"].as_str().unwrap_or_default();
                if id_raw.is_empty() {
                    continue;
                }
                let id = CapabilityId::from_str(id_raw)
                    .with_context(|| format!("unknown capability id in emission: {id_raw}"))?;
                if emission.data["provided"].as_bool().unwrap_or(false) {
                    provided_caps.push(id);
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
                } else {
                    required_caps.push(id);
                }
            }
            "connector_builtin" => {
                if let Some(id) = emission.data["id"].as_str() {
                    builtin_connectors.push(id.to_string());
                }
            }
            "connector_custom" => {
                let mut connector = serde_json::json!({
                    "id": emission.data["id"],
                    "displayNameKey": emission.data["displayNameKey"],
                    "baseUrl": emission.data["baseUrl"],
                    "credentialProviderId": emission.data["credentialProviderId"],
                    "operations": []
                });
                if let Some(auth) = emission.data.get("auth").filter(|v| !v.is_null()) {
                    if let Some(obj) = connector.as_object_mut() {
                        obj.insert("auth".to_string(), auth.clone());
                    }
                }
                custom_connectors.push(connector);
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
        manifest_version: MANIFEST_VERSION.to_string(),
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
            host: SDUI_SCHEMA_VERSION.to_string(),
            guest: SDUI_SCHEMA_VERSION.to_string(),
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

/// Finds the `portaki-emissions` directory of the module's latest compile.
///
/// Cargo keeps one build directory per fingerprint — per profile, target, SDK version, feature
/// set — so `target/` accumulates several emission trees, most of them stale. The right one is
/// the **most recently written**, judged on the files it holds: a directory's own mtime only
/// moves when an entry is created or removed, not when the macros rewrite a fragment in place.
///
/// This used to pick the last one *by path*. `release` sorts after `debug`, and a hash after
/// another, so a module could have its manifest regenerated from six-week-old emissions on every
/// build — the published version, the operations, the surfaces all silently stale.
///
/// Only the module's own crate is considered (`build/<crate>-<hash>/out/portaki-emissions`): in
/// a `target/` shared by a workspace, another member's emissions are just as fresh and wrong.
pub fn find_emissions_dir(module_root: &Path) -> Option<PathBuf> {
    let target = module_root.join("target");
    if !target.exists() {
        return None;
    }
    let crate_name = fs::read_to_string(module_root.join("Cargo.toml"))
        .ok()
        .and_then(|cargo| package_name(&cargo));

    // `target/<triple>/<profile>/build/<crate>-<hash>/out/portaki-emissions` is six levels deep;
    // walking further only reads compiled artifacts.
    let all: Vec<PathBuf> = WalkDir::new(&target)
        .max_depth(6)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_dir() && entry.file_name() == "portaki-emissions")
        .map(|entry| entry.path().to_path_buf())
        .collect();

    let own: Vec<&PathBuf> = match &crate_name {
        Some(name) => all.iter().filter(|dir| built_for(dir, name)).collect(),
        None => Vec::new(),
    };
    // Unknown crate name, or a layout we do not recognise: every tree competes, freshest wins.
    let pool: Vec<&PathBuf> = if own.is_empty() {
        all.iter().collect()
    } else {
        own
    };

    pool.into_iter()
        .max_by_key(|dir| last_written(dir))
        .cloned()
}

/// Whether `…/build/<crate>-<hash>/out/portaki-emissions` belongs to `crate_name`.
///
/// The hash is checked to be one: `ical-` prefixes both `ical-sync-9f3a…` and a crate named
/// `ical`, and only the remainder tells them apart.
fn built_for(emissions_dir: &Path, crate_name: &str) -> bool {
    let Some(build_dir) = emissions_dir.parent().and_then(Path::parent) else {
        return false;
    };
    let Some(dir_name) = build_dir.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    dir_name
        .strip_prefix(crate_name)
        .and_then(|rest| rest.strip_prefix('-'))
        .is_some_and(|hash| !hash.is_empty() && hash.chars().all(|c| c.is_ascii_hexdigit()))
}

/// The latest write inside an emission tree — its files; the directory itself only when empty.
///
/// Not the max of both: a directory recreated today would lend its date to fragments written
/// weeks ago, which is the very staleness this avoids.
fn last_written(dir: &Path) -> std::time::SystemTime {
    fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|entry| entry.metadata().ok()?.modified().ok())
        .max()
        .or_else(|| fs::metadata(dir).and_then(|meta| meta.modified()).ok())
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
}

/// The `[package]` name of a `Cargo.toml` — read in its own section, like the crate version.
fn package_name(cargo: &str) -> Option<String> {
    let mut in_package = false;
    for line in cargo.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_package = line == "[package]";
            continue;
        }
        if !in_package {
            continue;
        }
        if let Some(rest) = line.strip_prefix("name") {
            let rest = rest.trim_start().strip_prefix('=')?.trim();
            return rest
                .strip_prefix('"')
                .and_then(|rest| rest.strip_suffix('"'))
                .map(str::to_string);
        }
    }
    None
}

#[cfg(test)]
mod emissions_dir_tests {
    use super::*;
    use std::time::{Duration, SystemTime};

    fn emissions(root: &Path, rel: &str, written: SystemTime) -> PathBuf {
        let dir = root.join("target").join(rel).join("out/portaki-emissions");
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("module-x.json");
        fs::write(&file, "{}").unwrap();
        fs::File::options()
            .write(true)
            .open(&file)
            .unwrap()
            .set_modified(written)
            .unwrap();
        dir
    }

    fn module(name: &str) -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        fs::write(
            root.path().join("Cargo.toml"),
            format!("[package]\nname = \"{name}\"\nversion = \"0.4.1\"\n\n[dependencies]\nname = \"not-this\"\n"),
        )
        .unwrap();
        root
    }

    /// The case that shipped: a July release tree sorted after today's debug tree by path.
    #[test]
    fn the_freshest_tree_wins_over_the_last_by_path() {
        let root = module("ical-sync");
        let july = SystemTime::now() - Duration::from_secs(45 * 24 * 3600);
        emissions(
            root.path(),
            "wasm32-unknown-unknown/release/build/ical-sync-58e578d3c53c15cd",
            july,
        );
        let today = emissions(
            root.path(),
            "wasm32-unknown-unknown/debug/build/ical-sync-635b988b4e8d0e3c",
            SystemTime::now(),
        );

        assert_eq!(find_emissions_dir(root.path()), Some(today));
    }

    /// A shared `target/` holds other members' trees — as fresh, and wrong.
    #[test]
    fn another_crate_s_tree_is_ignored_even_when_fresher() {
        let root = module("ical-sync");
        let mine = emissions(
            root.path(),
            "wasm32-unknown-unknown/release/build/ical-sync-447cb326d508cae4",
            SystemTime::now() - Duration::from_secs(60),
        );
        emissions(
            root.path(),
            "wasm32-unknown-unknown/release/build/weather-0123abcd",
            SystemTime::now(),
        );
        emissions(
            root.path(),
            "wasm32-unknown-unknown/release/build/ical-00ff00ff",
            SystemTime::now(),
        );

        assert_eq!(find_emissions_dir(root.path()), Some(mine));
    }

    #[test]
    fn without_a_readable_crate_name_the_freshest_tree_still_wins() {
        let root = tempfile::tempdir().unwrap();
        emissions(
            root.path(),
            "debug/build/a-01",
            SystemTime::now() - Duration::from_secs(600),
        );
        let fresh = emissions(root.path(), "debug/build/b-02", SystemTime::now());

        assert_eq!(find_emissions_dir(root.path()), Some(fresh));
    }

    #[test]
    fn the_package_name_is_read_from_its_own_section() {
        assert_eq!(
            package_name("[dependencies]\nname = \"no\"\n[package]\nname = \"ical-sync\"\n")
                .as_deref(),
            Some("ical-sync")
        );
        assert_eq!(package_name("[package]\nname.workspace = true\n"), None);
    }
}
