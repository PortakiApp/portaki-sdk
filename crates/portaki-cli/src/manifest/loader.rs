//! Find a module's manifest, built or not.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use portaki_sdk::manifest::ModuleManifest;

use super::{collect_emissions, find_emissions_dir, generate_manifest};

/// Where `portaki build` drops the merged manifest.
pub const BUILT_MANIFEST: &str = "target/portaki/manifest.json";

/// Where the manifest we have just read comes from.
///
/// The caller tells the user which one it is: "what I am showing you comes from a build" and
/// "what I am showing you comes from the sources, there is no build yet" are not worth the same
/// — the second one may describe a module that does not compile.
pub enum Source {
    /// The file written by the last build.
    Built(PathBuf),
    /// The proc-macro emissions, without going through a build.
    Emissions,
}

/// Reads the module's manifest, from the build if there is one, from the emissions otherwise.
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
