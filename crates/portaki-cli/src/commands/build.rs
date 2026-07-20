//! `portaki build` — wasm build + manifest + i18n bundle.

use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result};
use clap::Parser;

use crate::manifest::{
    collect_emissions, find_emissions_dir, generate_manifest, write_manifest,
    write_migration_bundle, write_operations_bundle,
};
use crate::oci::pack;

#[derive(Debug, Parser)]
/// Arguments for `portaki build`.
pub struct BuildArgs {
    /// Build in release mode.
    #[arg(long)]
    pub release: bool,
    /// Skip `cargo build` (manifest-only refresh).
    #[arg(long)]
    pub manifest_only: bool,
}

/// Runs `portaki build`.
pub async fn run(args: BuildArgs) -> Result<()> {
    let module_root = std::env::current_dir().context("current_dir")?;
    let out_dir = module_root.join("target/portaki");
    std::fs::create_dir_all(&out_dir)?;

    let catalog_path = module_root.join("portaki.module.json");

    if !args.manifest_only {
        let mut cmd = Command::new("cargo");
        cmd.arg("build")
            .arg("--target")
            .arg("wasm32-unknown-unknown");
        if args.release {
            cmd.arg("--release");
        }
        let status = cmd.status().context("cargo build wasm32")?;
        if !status.success() {
            anyhow::bail!("cargo build failed");
        }
    }

    if let Some(emissions_dir) = find_emissions_dir(&module_root) {
        let emissions = collect_emissions(&emissions_dir)?;
        let i18n_dir = module_root.join("i18n");
        let supported = read_supported_locales(&i18n_dir)
            .unwrap_or_else(|| vec!["fr-FR".to_string(), "en-US".to_string()]);
        let default_locale = supported
            .first()
            .cloned()
            .unwrap_or_else(|| "fr-FR".to_string());

        let manifest = generate_manifest(&emissions, &default_locale, &supported)?;
        write_manifest(&manifest, &out_dir.join("manifest.json"))?;

        let schema_version = manifest
            .entities
            .iter()
            .map(|entity| entity.schema_version)
            .max()
            .unwrap_or(1);
        if let Some(bundle_path) =
            write_migration_bundle(&module_root, &out_dir, &manifest.id, schema_version)?
        {
            println!(
                "Wrote migrations bundle at {} (applied on module install to schema module_{})",
                bundle_path.display(),
                manifest.id.replace('-', "_")
            );
        }

        let module_version = catalog_module_version(&catalog_path).unwrap_or(manifest.version.clone());
        if let Some(bundle_path) = write_operations_bundle(
            &out_dir,
            &manifest.id,
            &module_version,
            schema_version,
            &manifest.entities,
        )? {
            println!(
                "Wrote operations bundle v2 at {} (schema.tables for typed-repo upsert)",
                bundle_path.display()
            );
        }

        bundle_i18n(&i18n_dir, &out_dir.join("i18n.tar.gz"))?;
    } else if !catalog_path.exists() {
        anyhow::bail!(
            "no portaki.module.json and no SDK emissions — add portaki_module!(...) or a catalog manifest"
        );
    }

    let publish_path = pack::assemble_publish_manifest(&module_root, &out_dir)?;

    println!(
        "Built publish manifest at {} (OCI layer source; edit portaki.module.json then rebuild)",
        publish_path.display()
    );
    Ok(())
}

fn catalog_module_version(catalog_path: &std::path::Path) -> Option<String> {
    let raw = std::fs::read_to_string(catalog_path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&raw).ok()?;
    value
        .get("version")
        .and_then(|version| version.as_str())
        .map(str::to_string)
}

fn read_supported_locales(i18n_dir: &PathBuf) -> Option<Vec<String>> {
    let entries = std::fs::read_dir(i18n_dir).ok()?;
    let mut locales = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.ends_with(".json") {
            locales.push(name.trim_end_matches(".json").to_string());
        }
    }
    if locales.is_empty() {
        None
    } else {
        locales.sort();
        Some(locales)
    }
}

fn bundle_i18n(i18n_dir: &PathBuf, dest: &PathBuf) -> Result<()> {
    if !i18n_dir.exists() {
        return Ok(());
    }
    let file = std::fs::File::create(dest)?;
    let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut archive = tar::Builder::new(encoder);
    for entry in std::fs::read_dir(i18n_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            archive.append_path_with_name(&path, path.file_name().unwrap())?;
        }
    }
    archive.finish()?;
    Ok(())
}
