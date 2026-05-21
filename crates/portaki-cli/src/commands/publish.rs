//! `portaki publish` — OCI push via ORAS-compatible layout (dry-run supported).

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;

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
}

/// Runs `portaki publish`.
pub async fn run(args: PublishArgs) -> Result<()> {
    let module_root = std::env::current_dir().context("current_dir")?;
    let artifact_dir = args
        .artifact_dir
        .unwrap_or_else(|| module_root.join("target/portaki"));

    oci::package_artifact(&artifact_dir).context("package OCI artifact")?;

    if args.dry_run {
        println!(
            "Dry-run: artifact ready at {} (registry: {})",
            artifact_dir.display(),
            args.registry
        );
        return Ok(());
    }

    oci::push_artifact(&artifact_dir, &args.registry)
        .await
        .context("push OCI artifact — install oras CLI and authenticate to Scaleway")?;
    println!("Published to {}", args.registry);
    Ok(())
}
