//! OCI artifact packaging and push (ORAS-compatible layout).

mod auth;
pub mod pack;

use std::path::Path;

use anyhow::{Context, Result};
use oci_distribution::client::{Client, Config};
use oci_distribution::Reference;

/// Validates that `portaki build` produced a publish manifest under `artifact_dir`.
pub fn package_artifact_with_root(_module_root: &Path, artifact_dir: &Path) -> Result<()> {
    let publish = pack::publish_manifest_path(artifact_dir);
    if !publish.exists() {
        anyhow::bail!("missing {} — run portaki build first", publish.display());
    }
    Ok(())
}

/// Pushes the module artifact to `registry` using `oci-distribution`.
///
/// Expects `portaki build` output under `artifact_dir`:
/// - `publish-manifest.json` (frozen catalog for OCI)
/// - `module_root/target/wasm32-unknown-unknown/release/*.wasm`
/// - `module_root/i18n/*.json` (optional)
///
/// Authentication: `GITHUB_TOKEN` / `GHCR_TOKEN` or Docker `config.json` for the registry host.
pub async fn push_artifact(
    module_root: &Path,
    artifact_dir: &Path,
    registry: &str,
) -> Result<String> {
    package_artifact_with_root(module_root, artifact_dir)?;

    let layers = pack::collect_push_layers(module_root, artifact_dir)?;
    let coords = pack::read_module_coordinates(module_root, artifact_dir)?;
    let image_ref = pack::image_reference(registry, &coords)?;
    let reference: Reference = image_ref
        .parse()
        .with_context(|| format!("invalid OCI reference: {image_ref}"))?;

    let image_layers = pack::layers_to_image_layers(&layers)?;
    let config = Config::new(
        br#"{}"#.to_vec(),
        "application/vnd.oci.empty.v1+json".to_string(),
        None,
    );

    let auth = auth::resolve_registry_auth(registry)?;
    let client = Client::default();
    let response = client
        .push(&reference, &image_layers, config, &auth, None)
        .await
        .context("OCI push to registry")?;

    Ok(response.manifest_url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_artifact_requires_publish_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let err = package_artifact_with_root(root, root).unwrap_err();
        assert!(err.to_string().contains("publish-manifest.json"));
    }
}
