//! `portaki lint` — validate manifest and i18n bundles.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use serde_json::from_reader;

use crate::manifest::collect_emissions;
use crate::manifest::{find_emissions_dir, generate_manifest, validate_manifest};
use crate::ui;
use portaki_sdk::manifest::ModuleManifest;

#[derive(Debug, Parser)]
/// Arguments for `portaki lint`.
pub struct LintArgs {
    /// Path to `manifest.json` (defaults to `target/portaki/manifest.json`).
    #[arg(long)]
    pub manifest: Option<PathBuf>,
}

/// Runs `portaki lint`.
pub fn run(args: LintArgs) -> Result<()> {
    ui::header("portaki lint");

    let module_root = std::env::current_dir().context("current_dir")?;
    let manifest_path = args
        .manifest
        .unwrap_or_else(|| module_root.join("target/portaki/manifest.json"));

    let reading = ui::step("reading the manifest");
    let manifest = if manifest_path.exists() {
        let file = std::fs::File::open(&manifest_path)?;
        let manifest = from_reader::<_, ModuleManifest>(file)?;
        reading.done(format!("read {}", manifest_path.display()));
        manifest
    } else if let Some(emissions_dir) = find_emissions_dir(&module_root) {
        let emissions = collect_emissions(&emissions_dir)?;
        let manifest = generate_manifest(
            &emissions,
            "fr-FR",
            &["fr-FR".to_string(), "en-US".to_string()],
        )?;
        reading.done("read the SDK emissions (no build output yet)");
        manifest
    } else {
        reading.fail("nothing to lint");
        anyhow::bail!("no manifest or emissions found — run portaki build first");
    };

    let checking = ui::step("checking capability ids, connector bindings, and i18n keys");
    validate_manifest(&manifest, &module_root.join("i18n")).map_err(|failure| {
        checking.fail("the manifest did not pass");
        failure
    })?;
    checking.done(format!("{} passes", manifest.id));
    ui::blank();
    Ok(())
}
