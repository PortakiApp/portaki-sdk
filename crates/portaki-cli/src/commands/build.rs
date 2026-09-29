//! `portaki build` — wasm build + manifest + i18n bundle.

use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result};
use clap::Parser;

use crate::manifest::{
    collect_emissions, find_emissions_dir_in, generate_manifest, write_manifest,
    write_migration_bundle, write_operations_bundle,
};
use crate::oci::pack;
use crate::{ui, workspace};

#[derive(Debug, Parser)]
/// Arguments for `portaki build`.
pub struct BuildArgs {
    /// Build in release mode.
    #[arg(long)]
    pub release: bool,
    /// Skip `cargo build` (manifest-only refresh).
    #[arg(long)]
    pub manifest_only: bool,
    /// In a repository holding several modules, the one to build.
    #[arg(long, conflicts_with = "all")]
    pub module: Option<String>,
    /// Build every module of the repository.
    #[arg(long)]
    pub all: bool,

    /// `release` chains straight on to `build`: a second header would look like two commands.
    #[arg(skip)]
    pub nested: bool,
}

/// Runs `portaki build`.
pub async fn run(args: BuildArgs) -> Result<()> {
    if !args.nested {
        ui::header(
            "portaki build",
            &crate::tr!(
                "Compile to wasm32, then turn the SDK's emissions into what the host reads.",
                "Compiler en wasm32, puis transformer les émissions du SDK en ce que l'hôte lit."
            ),
        );
    }
    // `release` has already moved into the module it is publishing.
    if args.nested {
        return build_here(&args).await;
    }
    let chosen = workspace::resolve(args.module.as_deref(), Some(args.all))?;
    for member in &chosen {
        if chosen.len() > 1 {
            ui::rule(&member.id);
        }
        workspace::enter(member)?;
        build_here(&args)
            .await
            .with_context(|| format!("build {}", member.id))?;
    }
    Ok(())
}

/// `portaki build` for the module in the current directory.
async fn build_here(args: &BuildArgs) -> Result<()> {
    let started = std::time::Instant::now();

    let module_root = std::env::current_dir().context("current_dir")?;
    let out_dir = module_root.join("target/portaki");
    std::fs::create_dir_all(&out_dir)?;

    let profile = if args.release { "release" } else { "debug" };

    if args.manifest_only {
        ui::skipped("cargo build skipped (--manifest-only)");
    } else {
        if let Some(written) = ensure_build_script(&module_root)? {
            ui::wrote("build.rs", written);
        }
        let mut cmd = Command::new("cargo");
        cmd.arg("build")
            .arg("--target")
            .arg("wasm32-unknown-unknown");
        if args.release {
            cmd.arg("--release");
        }
        ui::command(
            &format!("compiling wasm32-unknown-unknown ({profile})"),
            &mut cmd,
        )
        .context("cargo build wasm32")?;
    }

    refresh_outputs(&module_root)?;

    if !args.manifest_only {
        let coords = pack::read_module_coordinates(&module_root, &out_dir)?;
        // The profile that was just compiled — not the release one, which a debug build
        // never produced.
        reject_wasm_bindgen(&pack::find_wasm_artifact_in(
            &module_root,
            &coords.id,
            profile,
        )?)?;
    }

    ui::blank();
    ui::detail(crate::tr!(
        "built in {}",
        "compilé en {}",
        ui::elapsed(started.elapsed())
    ));
    if !args.nested {
        ui::next(&[
            (
                "portaki check",
                &crate::tr!(
                    "the gate portaki release applies: tests, manifest, texts",
                    "la porte que portaki release applique : tests, manifeste, textes"
                ),
            ),
            (
                "portaki dev --watch",
                &crate::tr!(
                    "deploy to the sandbox and see what a run does",
                    "déployer en sandbox et voir ce que fait une exécution"
                ),
            ),
        ]);
        ui::blank();
    }
    Ok(())
}

/// Writes a minimal `build.rs` when the module has none: without it Cargo gives no `OUT_DIR`, the
/// macros write nothing, and the wasm32 build stops on `portaki_module!`'s error.
/// An explicit `[package] build = …` is left exactly as it is.
fn ensure_build_script(module_root: &std::path::Path) -> Result<Option<&'static str>> {
    let path = module_root.join("build.rs");
    let cargo = std::fs::read_to_string(module_root.join("Cargo.toml")).unwrap_or_default();
    let declared = cargo.lines().any(|line| {
        line.trim_start().starts_with("build ") || line.trim_start().starts_with("build=")
    });
    if path.exists() || declared {
        return Ok(None);
    }
    std::fs::write(&path, BUILD_SCRIPT).context("write build.rs")?;
    Ok(Some("build.rs"))
}

/// What `portaki build` writes: nothing to do, only to exist.
const BUILD_SCRIPT: &str =
    "// Cargo gives the Portaki macros an OUT_DIR only when the crate has a build script.\n\
fn main() {}\n";

/// Refuses a wasm that expects `wasm-bindgen`.
///
/// The Extism host provides none of those imports: the module would load and then fail at run
/// time, far from here, with a message that does not name the cause. A dependency pulled in
/// without thinking is enough to make them appear — it happened, and the check had lived ever
/// since in one repository's bash script. It belongs in the build.
/// Turns the latest SDK emissions into what the host reads: `manifest.json`, migrations,
/// operations and i18n bundles, and the publish manifest.
///
/// Shared with `portaki dev`, which compiled but never ran this: it shipped whatever manifest
/// the last `portaki build` had left in `target/portaki/`, so a new query or surface stayed
/// invisible in the sandbox until someone thought of running `build` by hand.
pub fn refresh_outputs(module_root: &std::path::Path) -> Result<()> {
    refresh_outputs_from(module_root, &module_root.join("target"))
}

/// The same, reading the emissions from a given `target/` — where a workspace build wrote them.
pub fn refresh_outputs_from(module_root: &std::path::Path, target: &std::path::Path) -> Result<()> {
    let out_dir = module_root.join("target/portaki");
    std::fs::create_dir_all(&out_dir)?;
    let catalog_path = module_root.join("portaki.module.json");

    if let Some(emissions_dir) = find_emissions_dir_in(target, module_root) {
        let emissions = collect_emissions(&emissions_dir)?;
        let i18n_dir = module_root.join("i18n");
        let supported = read_supported_locales(&i18n_dir)
            .unwrap_or_else(|| vec!["fr-FR".to_string(), "en-US".to_string()]);
        let default_locale = supported
            .first()
            .cloned()
            .unwrap_or_else(|| "fr-FR".to_string());

        let sdk_features = pack::sdk_features(module_root)?;
        let mut manifest = generate_manifest(&emissions, &default_locale, &supported)?;
        crate::manifest::imply_storage(&mut manifest, &emissions, &sdk_features);
        let manifest_path = out_dir.join("manifest.json");
        write_manifest(&manifest, &manifest_path)?;
        crate::manifest::catalog::check_references(&emissions, &i18n_dir, &supported)?;
        let mut catalog =
            crate::manifest::catalog::catalog_defaults(&emissions, &i18n_dir, &supported);
        let permissions = crate::manifest::catalog::permissions(&emissions, &sdk_features);
        if !permissions.is_empty() {
            catalog["permissions"] = permissions.into();
        }
        std::fs::write(
            module_root.join(crate::manifest::catalog::BUILT_CATALOG),
            serde_json::to_string_pretty(&catalog)?,
        )?;
        ui::wrote("manifest", relative(&manifest_path, module_root));
        ui::detail(format!(
            "{} entities · {} locales · default {default_locale}",
            manifest.entities.len(),
            supported.len()
        ));

        let schema_version = manifest
            .entities
            .iter()
            .map(|entity| entity.schema_version)
            .max()
            .unwrap_or(1);
        if let Some(bundle_path) =
            write_migration_bundle(module_root, &out_dir, &manifest.id, schema_version)?
        {
            ui::wrote("migrations", relative(&bundle_path, module_root));
            ui::detail(format!(
                "applied on module install to schema module_{}",
                manifest.id.replace('-', "_")
            ));
        }

        let module_version =
            catalog_module_version(&catalog_path).unwrap_or(manifest.version.clone());
        if let Some(bundle_path) = write_operations_bundle(
            &out_dir,
            &manifest.id,
            &module_version,
            schema_version,
            &manifest.entities,
        )? {
            ui::wrote("operations", relative(&bundle_path, module_root));
            ui::detail("v2 — schema.tables for typed-repo upsert");
        }

        if let Some(bundle_path) = bundle_i18n(&i18n_dir, &out_dir.join("i18n.tar.gz"))? {
            ui::wrote("i18n", relative(&bundle_path, module_root));
            ui::detail(supported.join(" "));
        }
    } else if !catalog_path.exists() {
        anyhow::bail!(crate::tr!(
            "no SDK emissions — add portaki_module!(...) to lib.rs",
            "aucune émission du SDK — ajoutez portaki_module!(...) à lib.rs"
        ));
    }

    let publish_path = pack::assemble_publish_manifest(module_root, &out_dir)?;
    ui::wrote("publish", relative(&publish_path, module_root));
    ui::advice(crate::tr!(
        "written from the code — portaki_module!, #[surface], #[email] and the portaki-sdk features",
        "écrit depuis le code — portaki_module!, #[surface], #[email] et les features de portaki-sdk"
    ));
    Ok(())
}

fn reject_wasm_bindgen(wasm: &std::path::Path) -> Result<()> {
    let bytes = std::fs::read(wasm).with_context(|| format!("read {}", wasm.display()))?;
    // Looked for in the raw bytes rather than in a decoded import table: the name appears in
    // clear in the import section, and reading the whole format just for one string would cost
    // yet another wasm parser.
    if bytes
        .windows(WBINDGEN.len())
        .any(|window| window == WBINDGEN)
    {
        anyhow::bail!(
            "{} imports wasm-bindgen — the Extism host provides none of it, so the module \
             would load and then fail at run time. A dependency pulled it in: build with \
             --target wasm32-unknown-unknown only, and check what was added recently",
            wasm.display()
        );
    }
    Ok(())
}

/// The mark `wasm-bindgen` leaves in the import section.
const WBINDGEN: &[u8] = b"__wbindgen";

/// The path as one would retype it: from the module root, not from the root of the disk.
fn relative(path: &std::path::Path, root: &std::path::Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
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

fn bundle_i18n(i18n_dir: &PathBuf, dest: &PathBuf) -> Result<Option<PathBuf>> {
    if !i18n_dir.exists() {
        return Ok(None);
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
    Ok(Some(dest.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_build_script_is_written_once_and_a_declared_one_kept() {
        let module = tempfile::tempdir().unwrap();
        std::fs::write(
            module.path().join("Cargo.toml"),
            "[package]\nname = \"m\"\n",
        )
        .unwrap();
        assert_eq!(
            ensure_build_script(module.path()).unwrap(),
            Some("build.rs")
        );
        let written = std::fs::read_to_string(module.path().join("build.rs")).unwrap();
        assert!(written.contains("fn main() {}"), "{written}");
        assert_eq!(ensure_build_script(module.path()).unwrap(), None);

        let custom = tempfile::tempdir().unwrap();
        std::fs::write(
            custom.path().join("Cargo.toml"),
            "[package]\nname = \"m\"\nbuild = \"tools/build.rs\"\n",
        )
        .unwrap();
        assert_eq!(ensure_build_script(custom.path()).unwrap(), None);
        assert!(!custom.path().join("build.rs").exists());
    }

    /// The check that used to live in a bash script: a wasm that expects `wasm-bindgen` loads
    /// and then fails at run time, far from the build, with a message that does not name the
    /// cause.
    #[test]
    fn a_wasm_importing_wasm_bindgen_is_refused() {
        let directory = tempfile::tempdir().unwrap();
        let wasm = directory.path().join("module.wasm");
        let mut bytes = b"\0asm\x01\0\0\0".to_vec();
        bytes.extend_from_slice(b"__wbindgen_placeholder__");
        std::fs::write(&wasm, bytes).unwrap();

        let refusal = reject_wasm_bindgen(&wasm).unwrap_err().to_string();

        assert!(refusal.contains("wasm-bindgen"));
        assert!(refusal.contains("run time"));
    }

    #[test]
    fn a_plain_wasm_passes() {
        let directory = tempfile::tempdir().unwrap();
        let wasm = directory.path().join("module.wasm");
        std::fs::write(&wasm, b"\0asm\x01\0\0\0kv.get").unwrap();

        assert!(reject_wasm_bindgen(&wasm).is_ok());
    }
}
