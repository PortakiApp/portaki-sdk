//! Collects module files into OCI layers for push.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use oci_distribution::client::ImageLayer;
use serde::Deserialize;

const MANIFEST_MEDIA: &str = "application/vnd.portaki.manifest+json";
const WASM_MEDIA: &str = "application/wasm";
const I18N_MEDIA: &str = "application/vnd.portaki.i18n+json";

/// One blob to upload with its OCI media type.
#[derive(Debug, Clone)]
pub struct PushLayer {
    pub path: PathBuf,
    pub media_type: String,
}

/// Module coordinates read from `manifest.json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleCoordinates {
    pub id: String,
    pub version: String,
}

/// Parsed `target/portaki/manifest.json`.
#[derive(Debug, Deserialize)]
struct ArtifactManifest {
    id: String,
    version: String,
}

/// Reads module id/version from `portaki.module.json` (catalog) or build `manifest.json`.
pub fn read_module_coordinates(module_root: &Path, artifact_dir: &Path) -> Result<ModuleCoordinates> {
    let catalog_path = module_root.join("portaki.module.json");
    if catalog_path.exists() {
        let raw = std::fs::read_to_string(&catalog_path)
            .with_context(|| format!("read {}", catalog_path.display()))?;
        let manifest: ArtifactManifest =
            serde_json::from_str(&raw).context("parse portaki.module.json")?;
        return Ok(ModuleCoordinates {
            id: manifest.id,
            version: manifest.version,
        });
    }
    let manifest_path = artifact_dir.join("manifest.json");
    let raw = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("read {}", manifest_path.display()))?;
    let manifest: ArtifactManifest =
        serde_json::from_str(&raw).context("parse target/portaki/manifest.json")?;
    Ok(ModuleCoordinates {
        id: manifest.id,
        version: manifest.version,
    })
}

/// Builds the OCI image reference `registry/module_id:version`.
pub fn image_reference(registry: &str, coords: &ModuleCoordinates) -> Result<String> {
    let registry = registry.trim_end_matches('/');
    if registry.is_empty() {
        anyhow::bail!("registry must not be empty");
    }
    Ok(format!("{}/{}:{}", registry, coords.id, coords.version))
}

/// Discovers wasm + manifest + i18n layers under the module tree.
pub fn collect_push_layers(module_root: &Path, artifact_dir: &Path) -> Result<Vec<PushLayer>> {
    let coords = read_module_coordinates(module_root, artifact_dir)?;
    let mut layers = Vec::new();

    let catalog_manifest = module_root.join("portaki.module.json");
    let manifest_layer_path = if catalog_manifest.exists() {
        catalog_manifest
    } else {
        artifact_dir.join("manifest.json")
    };
    layers.push(PushLayer {
        path: manifest_layer_path,
        media_type: MANIFEST_MEDIA.to_string(),
    });

    let wasm_path = find_wasm_artifact(module_root, &coords.id)?;
    layers.push(PushLayer {
        path: wasm_path,
        media_type: WASM_MEDIA.to_string(),
    });

    let i18n_dir = module_root.join("i18n");
    if i18n_dir.is_dir() {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(&i18n_dir)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("json"))
            .collect();
        entries.sort();
        for path in entries {
            layers.push(PushLayer {
                path,
                media_type: I18N_MEDIA.to_string(),
            });
        }
    }

    Ok(layers)
}

/// Converts push layers to `oci-distribution` image layers (reads bytes from disk).
pub fn layers_to_image_layers(layers: &[PushLayer]) -> Result<Vec<ImageLayer>> {
    let mut image_layers = Vec::with_capacity(layers.len());
    for layer in layers {
        let data = std::fs::read(&layer.path)
            .with_context(|| format!("read layer {}", layer.path.display()))?;
        image_layers.push(ImageLayer::new(data, layer.media_type.clone(), None));
    }
    Ok(image_layers)
}

fn find_wasm_artifact(module_root: &Path, module_id: &str) -> Result<PathBuf> {
    let release_dir = module_root.join("target/wasm32-unknown-unknown/release");
    let candidates = [release_dir.join(format!("{module_id}.wasm"))];
    for candidate in &candidates {
        if candidate.exists() {
            return Ok(candidate.clone());
        }
    }

    if release_dir.is_dir() {
        let mut wasm_files: Vec<PathBuf> = std::fs::read_dir(&release_dir)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("wasm"))
            .collect();
        wasm_files.sort();
        if let Some(path) = wasm_files.into_iter().next() {
            return Ok(path);
        }
    }

    anyhow::bail!(
        "no wasm artifact under {} — run portaki build --release first",
        release_dir.display()
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn read_module_coordinates_parses_manifest() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("manifest.json"),
            r#"{"id":"weather","version":"0.2.0"}"#,
        )
        .unwrap();
        let coords = read_module_coordinates(dir.path(), dir.path()).unwrap();
        assert_eq!(
            coords,
            ModuleCoordinates {
                id: "weather".to_string(),
                version: "0.2.0".to_string(),
            }
        );
    }

    #[test]
    fn read_module_coordinates_prefers_portaki_module_json() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("portaki.module.json"),
            r#"{"id":"weather","version":"1.3.2"}"#,
        )
        .unwrap();
        fs::write(
            dir.path().join("manifest.json"),
            r#"{"id":"other","version":"0.1.0"}"#,
        )
        .unwrap();
        let coords = read_module_coordinates(dir.path(), dir.path()).unwrap();
        assert_eq!(
            coords,
            ModuleCoordinates {
                id: "weather".to_string(),
                version: "1.3.2".to_string(),
            }
        );
    }

    #[test]
    fn image_reference_formats_registry_tag() {
        let coords = ModuleCoordinates {
            id: "weather".into(),
            version: "0.2.0".into(),
        };
        let reference = image_reference("rg.fr-par.scw.cloud/portaki-modules", &coords).unwrap();
        assert_eq!(
            reference,
            "rg.fr-par.scw.cloud/portaki-modules/weather:0.2.0"
        );
    }

    #[test]
    fn collect_push_layers_includes_manifest_wasm_and_i18n() {
        let root = tempdir().unwrap();
        let artifact = root.path().join("target/portaki");
        fs::create_dir_all(&artifact).unwrap();
        fs::write(
            artifact.join("manifest.json"),
            r#"{"id":"weather","version":"0.1.0"}"#,
        )
        .unwrap();

        let wasm_dir = root.path().join("target/wasm32-unknown-unknown/release");
        fs::create_dir_all(&wasm_dir).unwrap();
        fs::write(wasm_dir.join("weather.wasm"), b"\0asm").unwrap();

        let i18n = root.path().join("i18n");
        fs::create_dir_all(&i18n).unwrap();
        fs::write(i18n.join("en-US.json"), "{}").unwrap();

        let layers = collect_push_layers(root.path(), &artifact).unwrap();
        assert_eq!(layers.len(), 3);
        assert_eq!(layers[0].media_type, MANIFEST_MEDIA);
        assert_eq!(layers[1].media_type, WASM_MEDIA);
        assert_eq!(layers[2].media_type, I18N_MEDIA);
    }
}
