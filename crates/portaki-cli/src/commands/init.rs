//! `portaki init` — scaffold a module from templates.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::{Parser, ValueEnum};
use include_dir::{include_dir, Dir};

use crate::ui;

/// The scaffolding, compiled into the binary.
///
/// Read from disk, it resolved against this crate's source directory — a path that exists in a
/// checkout of this repository and nowhere else, so `cargo install portaki-cli` produced a
/// command that could not scaffold anything.
static TEMPLATES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates");

#[derive(Debug, Clone, ValueEnum)]
/// Template kind for `portaki init`.
pub enum InitTemplate {
    /// Default module with entity, surfaces, and i18n bundles.
    Default,
    /// Minimal empty module skeleton.
    Empty,
}

#[derive(Debug, Parser)]
/// Arguments for `portaki init`.
pub struct InitArgs {
    /// Module name (kebab-case recommended).
    pub name: String,
    /// Template to use.
    #[arg(long, value_enum, default_value_t = InitTemplate::Default)]
    pub template: InitTemplate,
    /// Output directory (defaults to `./{name}`).
    #[arg(long)]
    pub path: Option<PathBuf>,
}

/// Runs `portaki init`.
pub fn run(args: InitArgs) -> Result<()> {
    ui::header(
        "portaki init",
        "Scaffold a module crate — buildable, runnable in the sandbox, publishable.",
    );

    let dest = args
        .path
        .clone()
        .unwrap_or_else(|| PathBuf::from(&args.name));

    if dest.exists() {
        bail!("destination already exists: {}", dest.display());
    }

    let template_dir = TEMPLATES
        .get_dir(directory(&args.template))
        .with_context(|| {
            format!(
                "template missing from this build: {}",
                label(&args.template)
            )
        })?;

    let scaffolding = ui::step(format!(
        "scaffolding {} from the {} template",
        args.name,
        label(&args.template)
    ));
    copy_template(template_dir, &dest, &args.name)?;
    scaffolding.done(format!("created {}", dest.display()));

    describe(&args.template);
    ui::next(&[
        (
            &format!("cd {}", dest.display()),
            "everything below runs from the module root",
        ),
        (
            "portaki build",
            "compile to wasm32 and assemble the manifest",
        ),
        (
            "portaki dev --watch",
            "run it in the hosted sandbox on every save",
        ),
    ]);
    ui::blank();
    Ok(())
}

/// Ce qui vient d'être écrit, et à quoi chaque morceau sert.
///
/// Un squelette qu'on découvre fichier par fichier se lit mal : `ids.rs` et `i18n/` n'ont de
/// sens que l'un par rapport à l'autre, et rien dans leur nom ne le dit.
fn describe(template: &InitTemplate) {
    let mut rows = vec![
        ("src/lib.rs", "the module — entity, capability, manifest"),
        ("src/ids.rs", "typed surface and operation ids"),
    ];
    if matches!(template, InitTemplate::Default) {
        rows.push(("src/host/", "surfaces the host dashboard renders"));
        rows.push(("src/guest/", "surfaces the guest booklet renders"));
    }
    rows.push((
        "i18n/*.json",
        "one file per locale — the keys ids.rs points at",
    ));
    rows.push(("Cargo.toml", "wired to portaki-sdk, cdylib for wasm32"));

    ui::list("what you got", &rows);
}

fn label(template: &InitTemplate) -> &'static str {
    match template {
        InitTemplate::Default => "default",
        InitTemplate::Empty => "empty",
    }
}

fn directory(template: &InitTemplate) -> &'static str {
    match template {
        InitTemplate::Default => "default-module",
        InitTemplate::Empty => "empty-module",
    }
}

/// Writes an embedded directory out, rendering each file on the way.
fn copy_template(source: &Dir<'_>, dest: &Path, module_name: &str) -> Result<()> {
    fs::create_dir_all(dest).with_context(|| format!("create {}", dest.display()))?;

    for file in source.files() {
        let name = file
            .path()
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default();
        // `Cargo.toml.template` would otherwise make the scaffolded crate a cargo package the
        // moment it is written, and cargo would read it while it still holds placeholders.
        let name = name.strip_suffix(".template").unwrap_or(&name).to_string();
        let target = dest.join(&name);

        let text = file
            .contents_utf8()
            .with_context(|| format!("template {} is not UTF-8", file.path().display()))?;
        // The CLI's version is the SDK it was published with: a scaffolded module compiles
        // against the SDK this command knows, not against whatever is newest.
        let rendered = text
            .replace("{{MODULE_NAME}}", module_name)
            .replace("{{SDK_VERSION}}", env!("CARGO_PKG_VERSION"));
        fs::write(&target, rendered).with_context(|| format!("write {}", target.display()))?;
    }

    for child in source.dirs() {
        let name = child
            .path()
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default();
        copy_template(child, &dest.join(name), module_name)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Both templates have to be in the binary, or `init` only fails for whoever installed it.
    #[test]
    fn every_template_is_embedded() {
        for template in [InitTemplate::Default, InitTemplate::Empty] {
            let dir = TEMPLATES
                .get_dir(directory(&template))
                .expect("template embedded");
            assert!(dir.files().count() + dir.dirs().count() > 0);
        }
    }

    #[test]
    fn a_scaffolded_module_carries_its_name_and_the_sdk_version() {
        let dest = std::env::temp_dir().join(format!("portaki-init-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dest);

        copy_template(
            TEMPLATES.get_dir("default-module").expect("template"),
            &dest,
            "concierge",
        )
        .expect("scaffold");

        let cargo = fs::read_to_string(dest.join("Cargo.toml")).expect("Cargo.toml written");
        assert!(cargo.contains("name = \"concierge\""));
        assert!(cargo.contains(env!("CARGO_PKG_VERSION")));
        assert!(!cargo.contains("{{"));
        // Nested and dot directories come out too — the wasm rustflags live in one of them.
        assert!(dest.join("src/host/mod.rs").exists());
        assert!(dest.join(".cargo/config.toml").exists());
        assert!(!dest.join("Cargo.toml.template").exists());

        fs::remove_dir_all(&dest).ok();
    }
}
