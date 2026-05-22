//! `portaki publish` — OCI push via `oci-distribution` (ORAS-compatible layout).
//!
//! Always runs `portaki build --release` first (unless `--skip-build`) so the OCI catalog layer
//! comes from `target/portaki/publish-manifest.json`, not a hand-edited repo file at publish time.
//!
//! Authenticates with `SCW_SECRET_KEY` (Scaleway, username `nologin`) or Docker `~/.docker/config.json`.

use std::path::PathBuf;

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
