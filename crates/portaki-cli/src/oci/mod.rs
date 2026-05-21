//! OCI artifact packaging helpers.

use std::path::Path;

use anyhow::Result;

/// Validates that required artifact files exist under `artifact_dir`.
pub fn package_artifact(artifact_dir: &Path) -> Result<()> {
    let manifest = artifact_dir.join("manifest.json");
    if !manifest.exists() {
        anyhow::bail!("missing {} — run portaki build first", manifest.display());
    }
    Ok(())
}

/// Pushes the artifact to `registry` using the `oras` CLI when available.
pub async fn push_artifact(artifact_dir: &Path, registry: &str) -> Result<()> {
    let _ = (artifact_dir, registry);
    anyhow::bail!(
        "OCI push requires the oras CLI and registry credentials — use --dry-run until CI wiring is complete"
    );
}
