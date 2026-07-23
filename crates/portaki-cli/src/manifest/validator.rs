//! Manifest and i18n validation for `portaki lint`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use anyhow::{bail, Result};
use portaki_sdk::capability::is_known;
use portaki_sdk::manifest::ModuleManifest;
use serde_json::Value;

/// Validates a generated manifest and locale bundles.
pub fn validate_manifest(manifest: &ModuleManifest, i18n_dir: &Path) -> Result<()> {
    for capability_id in manifest
        .capabilities
        .required
        .iter()
        .chain(manifest.capabilities.optional.iter().map(|c| &c.id))
        .chain(manifest.capabilities.provided.iter())
    {
        // Typed as [`CapabilityId`]; still guard for defensive CLI use.
        if !is_known(capability_id.as_str()) {
            bail!("unknown capability id in manifest: {capability_id}");
        }
    }

    let mut keys_by_locale: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for locale in &manifest.i18n.supported {
        let path = i18n_dir.join(format!("{locale}.json"));
        if !path.exists() {
            bail!(
                "missing i18n bundle for locale {locale}: {}",
                path.display()
            );
        }
        let text = fs::read_to_string(&path)?;
        let json: Value = serde_json::from_str(&text)?;
        let keys = json
            .as_object()
            .map(|map| map.keys().cloned().collect::<BTreeSet<_>>())
            .unwrap_or_default();
        keys_by_locale.insert(locale.clone(), keys);
    }

    let reference_locale = &manifest.i18n.default;
    let reference_keys = keys_by_locale
        .get(reference_locale)
        .cloned()
        .unwrap_or_default();

    for locale in &manifest.i18n.supported {
        if locale == reference_locale {
            continue;
        }
        let keys = keys_by_locale.get(locale).cloned().unwrap_or_default();
        for key in &reference_keys {
            if !keys.contains(key) {
                bail!("missing i18n key '{key}' in locale {locale}");
            }
        }
    }

    collect_i18n_keys(manifest, &reference_keys)?;

    Ok(())
}

fn collect_i18n_keys(manifest: &ModuleManifest, reference_keys: &BTreeSet<String>) -> Result<()> {
    let mut required_keys = BTreeSet::new();
    if manifest.display_name.starts_with("i18n:") {
        required_keys.insert(
            manifest
                .display_name
                .trim_start_matches("i18n:")
                .to_string(),
        );
    }
    if manifest.description.starts_with("i18n:") {
        required_keys.insert(manifest.description.trim_start_matches("i18n:").to_string());
    }
    for optional in &manifest.capabilities.optional {
        if optional.purpose_key.starts_with("i18n:") {
            required_keys.insert(optional.purpose_key.trim_start_matches("i18n:").to_string());
        }
        if optional.fallback_key.starts_with("i18n:") {
            required_keys.insert(
                optional
                    .fallback_key
                    .trim_start_matches("i18n:")
                    .to_string(),
            );
        }
    }

    for key in required_keys {
        if !reference_keys.contains(&key) {
            bail!("manifest references missing i18n key: {key}");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use portaki_sdk::manifest::{
        ManifestAuthor, ManifestCapabilities, ManifestConnectors, ManifestEvents, ManifestI18n,
        ManifestSurfaces, ModuleManifest, UiSchemaVersions,
    };

    #[test]
    fn rejects_unknown_capability_string() {
        assert!(!is_known("not.a.real.capability"));
    }

    #[test]
    fn accepts_known_required_capability() {
        let manifest = ModuleManifest {
            manifest_version: "1".into(),
            id: "test".into(),
            version: "0.1.0".into(),
            display_name: "module.displayName".into(),
            description: "module.description".into(),
            author: ManifestAuthor {
                name: "Syntax Labs".into(),
                url: None,
                support_email: None,
            },
            ui_schema: UiSchemaVersions {
                host: "1".into(),
                guest: "1".into(),
            },
            capabilities: ManifestCapabilities {
                required: vec![portaki_sdk::capability::CapabilityId::Storage],
                optional: vec![],
                provided: vec![],
            },
            connectors: ManifestConnectors::default(),
            entities: vec![],
            surfaces: ManifestSurfaces::default(),
            queries: vec![],
            commands: vec![],
            events: ManifestEvents::default(),
            i18n: ManifestI18n {
                default: "fr-FR".into(),
                supported: vec!["fr-FR".into()],
            },
        };

        let temp = tempfile::tempdir().expect("tempdir");
        let i18n = temp.path().join("i18n");
        std::fs::create_dir_all(&i18n).expect("dir");
        std::fs::write(
            i18n.join("fr-FR.json"),
            r#"{"module.displayName":"x","module.description":"y"}"#,
        )
        .expect("write");

        assert!(validate_manifest(&manifest, &i18n).is_ok());
    }
}
