//! Collects module files into OCI layers for push.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use oci_distribution::client::ImageLayer;
use serde::Deserialize;

const MANIFEST_MEDIA: &str = "application/vnd.portaki.manifest+json";
const WASM_MEDIA: &str = "application/wasm";
const I18N_MEDIA: &str = "application/vnd.portaki.i18n+json";

/// OCI catalog layer written by `portaki build` — never push repo-root `portaki.module.json` directly.
pub const PUBLISH_MANIFEST: &str = "publish-manifest.json";

/// One blob to upload with its OCI media type.
#[derive(Debug, Clone)]
pub struct PushLayer {
    pub path: PathBuf,
    pub media_type: String,
}

/// Module coordinates read from the publish manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleCoordinates {
    pub id: String,
    pub version: String,
}

/// Parsed publish / SDK manifest (`id` + `version` for OCI tag).
#[derive(Debug, Deserialize)]
struct ArtifactManifest {
    id: String,
    version: String,
}

/// Path to the frozen manifest produced by `portaki build`.
pub fn publish_manifest_path(artifact_dir: &Path) -> PathBuf {
    artifact_dir.join(PUBLISH_MANIFEST)
}

/// Assembles `target/portaki/publish-manifest.json` from sources (catalog + optional SDK build output).
pub fn assemble_publish_manifest(module_root: &Path, artifact_dir: &Path) -> Result<PathBuf> {
    fs::create_dir_all(artifact_dir).context("create artifact dir")?;
    let dest = publish_manifest_path(artifact_dir);
    let catalog_path = module_root.join("portaki.module.json");
    let sdk_path = artifact_dir.join("manifest.json");

    if catalog_path.exists() {
        fs::copy(&catalog_path, &dest)
            .with_context(|| format!("copy {} -> {}", catalog_path.display(), dest.display()))?;
        return Ok(dest);
    }

    if sdk_path.exists() {
        fs::copy(&sdk_path, &dest)
            .with_context(|| format!("copy {} -> {}", sdk_path.display(), dest.display()))?;
        return Ok(dest);
    }

    anyhow::bail!(
        "missing portaki.module.json or {} — run portaki build first",
        sdk_path.display()
    );
}

/// Reads module id/version from `publish-manifest.json` under `artifact_dir`.
pub fn read_module_coordinates(
    _module_root: &Path,
    artifact_dir: &Path,
) -> Result<ModuleCoordinates> {
    let manifest_path = publish_manifest_path(artifact_dir);
    let raw = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("read {}", manifest_path.display()))?;
    let manifest: ArtifactManifest =
        serde_json::from_str(&raw).context("parse publish-manifest.json")?;
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

/// Discovers wasm + publish manifest + i18n layers under the module tree.
pub fn collect_push_layers(module_root: &Path, artifact_dir: &Path) -> Result<Vec<PushLayer>> {
    let coords = read_module_coordinates(module_root, artifact_dir)?;
    let mut layers = Vec::new();

    let manifest_layer_path = publish_manifest_path(artifact_dir);
    if !manifest_layer_path.exists() {
        anyhow::bail!(
            "missing {} — run portaki build before publish",
            manifest_layer_path.display()
        );
    }
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
    fn assemble_publish_manifest_copies_catalog_source() {
        let root = tempdir().unwrap();
        fs::write(
            root.path().join("portaki.module.json"),
            r#"{"id":"weather","version":"1.3.2"}"#,
        )
        .unwrap();
        let artifact = root.path().join("target/portaki");
        let path = assemble_publish_manifest(root.path(), &artifact).unwrap();
        assert_eq!(path, artifact.join(PUBLISH_MANIFEST));
        let raw = fs::read_to_string(&path).unwrap();
        assert!(raw.contains("\"version\":\"1.3.2\""));
    }

    #[test]
    fn assemble_publish_manifest_copies_sdk_manifest_when_no_catalog() {
        let root = tempdir().unwrap();
        let artifact = root.path().join("target/portaki");
        fs::create_dir_all(&artifact).unwrap();
        fs::write(
            artifact.join("manifest.json"),
            r#"{"id":"weather","version":"0.2.0"}"#,
        )
        .unwrap();
        assemble_publish_manifest(root.path(), &artifact).unwrap();
        let raw = fs::read_to_string(artifact.join(PUBLISH_MANIFEST)).unwrap();
        assert!(raw.contains("\"version\":\"0.2.0\""));
    }

    #[test]
    fn read_module_coordinates_reads_publish_manifest_only() {
        let dir = tempdir().unwrap();
        let artifact = dir.path().join("target/portaki");
        fs::create_dir_all(&artifact).unwrap();
        fs::write(
            dir.path().join("portaki.module.json"),
            r#"{"id":"stale","version":"0.0.1"}"#,
        )
        .unwrap();
        fs::write(
            artifact.join(PUBLISH_MANIFEST),
            r#"{"id":"weather","version":"0.2.0"}"#,
        )
        .unwrap();
        let coords = read_module_coordinates(dir.path(), &artifact).unwrap();
        assert_eq!(
            coords,
            ModuleCoordinates {
                id: "weather".to_string(),
                version: "0.2.0".to_string(),
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
    fn collect_push_layers_uses_publish_manifest_not_repo_catalog() {
        let root = tempdir().unwrap();
        fs::write(
            root.path().join("portaki.module.json"),
            r#"{"id":"weather","version":"9.9.9"}"#,
        )
        .unwrap();
        let artifact = root.path().join("target/portaki");
        fs::create_dir_all(&artifact).unwrap();
        fs::write(
            artifact.join(PUBLISH_MANIFEST),
            r#"{"id":"weather","version":"0.1.0"}"#,
        )
        .unwrap();

        let wasm_dir = root.path().join("target/wasm32-unknown-unknown/release");
        fs::create_dir_all(&wasm_dir).unwrap();
        fs::write(wasm_dir.join("weather.wasm"), b"\0asm").unwrap();

        let layers = collect_push_layers(root.path(), &artifact).unwrap();
        assert_eq!(layers.len(), 2);
        assert_eq!(layers[0].path, artifact.join(PUBLISH_MANIFEST));
        assert_eq!(layers[0].media_type, MANIFEST_MEDIA);
    }
}
