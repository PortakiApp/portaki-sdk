//! Retrouver le manifeste d'un module, construit ou non.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use portaki_sdk::manifest::ModuleManifest;

use super::{collect_emissions, find_emissions_dir, generate_manifest};

/// Là où `portaki build` dépose le manifeste fusionné.
pub const BUILT_MANIFEST: &str = "target/portaki/manifest.json";

/// D'où vient le manifeste qu'on vient de lire.
///
/// L'appelant le dit à l'utilisateur : « ce que je te montre vient d'un build » et « ce que je
/// te montre vient des sources, il n'y a pas encore de build » ne se valent pas — la seconde
/// peut décrire un module qui ne compile pas.
pub enum Source {
    /// Le fichier écrit par le dernier build.
    Built(PathBuf),
    /// Les émissions du proc-macro, sans passer par un build.
    Emissions,
}

/// Lit le manifeste du module, du build s'il existe, des émissions sinon.
pub fn load(module_root: &Path, explicit: Option<PathBuf>) -> Result<(ModuleManifest, Source)> {
    let path = explicit.unwrap_or_else(|| module_root.join(BUILT_MANIFEST));

    if path.exists() {
        let file =
            std::fs::File::open(&path).with_context(|| format!("open {}", path.display()))?;
        let manifest = serde_json::from_reader::<_, ModuleManifest>(file)
            .with_context(|| format!("parse {}", path.display()))?;
        return Ok((manifest, Source::Built(path)));
    }

    let emissions_dir = find_emissions_dir(module_root)
        .context("no manifest and no SDK emissions — run portaki build, from the module root")?;
    let emissions = collect_emissions(&emissions_dir)?;
    let manifest = generate_manifest(
        &emissions,
        "fr-FR",
        &["fr-FR".to_string(), "en-US".to_string()],
    )?;
    Ok((manifest, Source::Emissions))
}
