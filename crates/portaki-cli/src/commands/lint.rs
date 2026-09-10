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
    ui::header(
        "portaki lint",
        "Check that everything the manifest names actually resolves.",
    );

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
        reading.abandon();
        anyhow::bail!("no manifest or emissions found — run portaki build first");
    };

    let checking = ui::step("checking capability ids, connector bindings, and i18n keys");
    validate_manifest(&manifest, &module_root.join("i18n")).map_err(|failure| {
        checking.abandon();
        failure
    })?;
    assert_versions_agree(&module_root, &manifest).map_err(|failure| {
        checking.abandon();
        failure
    })?;
    checking.done(format!("{} passes", manifest.id));
    ui::detail("capability ids, connector bindings and i18n keys all resolve");
    ui::blank();
    Ok(())
}

/// La crate et le manifeste doivent annoncer la même version.
///
/// `release-please` incrémente les deux ; si l'un des deux passe à travers, un artefact part
/// sous un numéro que rien d'autre ne porte, et la version publiée cesse de désigner le code
/// qu'elle contient. Le contrôle vivait dans le script bash d'un dépôt — il appartient au lint.
fn assert_versions_agree(module_root: &std::path::Path, manifest: &ModuleManifest) -> Result<()> {
    let cargo = module_root.join("Cargo.toml");
    let Ok(text) = std::fs::read_to_string(&cargo) else {
        // Pas de crate ici : un manifeste peut être linté seul.
        return Ok(());
    };
    let Some(declared) = crate_version(&text) else {
        return Ok(());
    };
    if declared == manifest.version {
        return Ok(());
    }
    anyhow::bail!(
        "Cargo.toml says {declared} and the manifest says {} — a release bumps both, so one of \
         them was missed",
        manifest.version
    )
}

/// La version de `[package]`, sans analyseur TOML pour un champ.
///
/// Lue dans sa seule section : une `version` de dépendance ne doit pas passer pour celle de la
/// crate. Une version héritée de l'espace de travail n'est pas lisible ici, et se lit comme
/// absente plutôt que comme un désaccord.
fn crate_version(cargo: &str) -> Option<String> {
    let mut in_package = false;
    for line in cargo.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_package = line == "[package]";
            continue;
        }
        if !in_package {
            continue;
        }
        if let Some(rest) = line.strip_prefix("version") {
            let rest = rest.trim_start().strip_prefix('=')?.trim();
            return rest
                .strip_prefix('"')
                .and_then(|rest| rest.strip_suffix('"'))
                .map(str::to_string);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_crate_version_is_read_from_its_own_section() {
        let cargo = r#"
[package]
name = "weather"
version = "0.3.24"

[dependencies]
serde = { version = "1", features = ["derive"] }
"#;

        assert_eq!(crate_version(cargo).as_deref(), Some("0.3.24"));
    }

    /// Une `version` de dépendance ne doit pas passer pour celle de la crate.
    #[test]
    fn a_dependency_version_is_not_mistaken_for_the_crate() {
        let cargo = "[dependencies]\nserde = { version = \"1\" }\n";

        assert!(crate_version(cargo).is_none());
    }

    #[test]
    fn a_workspace_inherited_version_is_left_alone() {
        let cargo = "[package]\nname = \"weather\"\nversion.workspace = true\n";

        assert!(crate_version(cargo).is_none());
    }
}
