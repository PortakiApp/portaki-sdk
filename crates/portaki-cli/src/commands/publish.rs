//! `portaki publish` — OCI push via `oci-distribution` (ORAS-compatible layout).
//!
//! Always runs `portaki build --release` first (unless `--skip-build`) so the OCI catalog layer
//! comes from `target/portaki/publish-manifest.json`, not a hand-edited repo file at publish time.
//!
//! Authenticates with `SCW_SECRET_KEY` (Scaleway, username `nologin`) or Docker `~/.docker/config.json`.
//!
//! Set `PORTAKI_PUBLISH_VERSION` (e.g. from CI git tag `*-vX.Y.Z`) to fail fast if `publish-manifest.json`
//! version does not match.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Parser;

use crate::commands::build::{self, BuildArgs};
use crate::oci;

#[derive(Debug, Parser)]
/// Arguments for `portaki publish`.
pub struct PublishArgs {
    /// OCI registry (Scaleway Container Registry).
    #[arg(long, default_value = "rg.fr-par.scw.cloud/portaki-modules")]
    pub registry: String,
    /// Validate packaging without pushing.
    #[arg(long)]
    pub dry_run: bool,
    /// Artifact directory (defaults to `target/portaki`).
    #[arg(long)]
    pub artifact_dir: Option<PathBuf>,
    /// Skip the implicit `portaki build --release` (not recommended).
    #[arg(long)]
    pub skip_build: bool,
}

/// Runs `portaki publish`.
pub async fn run(args: PublishArgs) -> Result<()> {
    let module_root = std::env::current_dir().context("current_dir")?;
    let artifact_dir = args
        .artifact_dir
        .clone()
        .unwrap_or_else(|| module_root.join("target/portaki"));

    if !args.skip_build {
        build::run(BuildArgs {
            release: true,
            manifest_only: false,
        })
        .await
        .context("portaki build --release before publish")?;
    }

    oci::package_artifact_with_root(&module_root, &artifact_dir).context("package OCI artifact")?;
    assert_publish_version_matches_env(&module_root, &artifact_dir)?;

    if args.dry_run {
        println!(
            "Dry-run: artifact ready at {} (registry: {})",
            artifact_dir.display(),
            args.registry
        );
        return Ok(());
    }

    let manifest_url = oci::push_artifact(&module_root, &artifact_dir, &args.registry)
        .await
        .context("push OCI artifact — set SCW_SECRET_KEY or docker login")?;
    println!("Published to {} ({})", args.registry, manifest_url);
    Ok(())
}

fn assert_publish_version_matches_env(module_root: &Path, artifact_dir: &Path) -> Result<()> {
    let expected = match std::env::var("PORTAKI_PUBLISH_VERSION") {
        Ok(value) => value,
        Err(_) => return Ok(()),
    };
    let expected = expected.trim();
    if expected.is_empty() {
        return Ok(());
    }
    let coords = oci::pack::read_module_coordinates(module_root, artifact_dir)?;
    if coords.version == expected {
        return Ok(());
    }
    anyhow::bail!(
        "publish-manifest version {} does not match PORTAKI_PUBLISH_VERSION={} — \
         align Cargo.toml with the git tag and rebuild (portaki build --release)",
        coords.version,
        expected
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use tempfile::tempdir;

    #[test]
    fn assert_publish_version_matches_env_accepts_matching_version() {
        let root = tempdir().unwrap();
        let artifact = root.path().join("target/portaki");
        fs::create_dir_all(&artifact).unwrap();
        fs::write(
            artifact.join(oci::pack::PUBLISH_MANIFEST),
            r#"{"id":"weather","version":"0.2.1"}"#,
        )
        .unwrap();
        unsafe {
            std::env::set_var("PORTAKI_PUBLISH_VERSION", "0.2.1");
        }
        assert_publish_version_matches_env(root.path(), &artifact).unwrap();
        unsafe {
            std::env::remove_var("PORTAKI_PUBLISH_VERSION");
        }
    }

    #[test]
    fn assert_publish_version_matches_env_rejects_mismatch() {
        let root = tempdir().unwrap();
        let artifact = root.path().join("target/portaki");
        fs::create_dir_all(&artifact).unwrap();
        fs::write(
            artifact.join(oci::pack::PUBLISH_MANIFEST),
            r#"{"id":"weather","version":"0.1.0"}"#,
        )
        .unwrap();
        unsafe {
            std::env::set_var("PORTAKI_PUBLISH_VERSION", "0.2.1");
        }
        let err = assert_publish_version_matches_env(root.path(), &artifact).unwrap_err();
        assert!(err.to_string().contains("0.1.0"));
        assert!(err.to_string().contains("0.2.1"));
        unsafe {
            std::env::remove_var("PORTAKI_PUBLISH_VERSION");
        }
    }
}
