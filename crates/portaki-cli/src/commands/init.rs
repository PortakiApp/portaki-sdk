//! `portaki init` — scaffold a module from templates.

use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::{Parser, ValueEnum};

use crate::ui;

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

    let template_dir = match args.template {
        InitTemplate::Default => template_root().join("default-module"),
        InitTemplate::Empty => template_root().join("empty-module"),
    };

    if !template_dir.exists() {
        bail!(
            "template not found: {} (run from portaki-sdk checkout)",
            template_dir.display()
        );
    }

    let scaffolding = ui::step(format!(
        "scaffolding {} from the {} template",
        args.name,
        label(&args.template)
    ));
    copy_template(&template_dir, &dest, &args.name)?;
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

fn template_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../templates")
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../templates"))
}

fn copy_template(source: &PathBuf, dest: &PathBuf, module_name: &str) -> Result<()> {
    fs::create_dir_all(dest).with_context(|| format!("create {}", dest.display()))?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();
        let target = dest.join(&*name);
        if entry.file_type()?.is_dir() {
            copy_template(&entry.path(), &target, module_name)?;
            continue;
        }
        let source_path = entry.path();
        let mut target_name = name.to_string();
        if target_name.ends_with(".template") {
            target_name = target_name.trim_end_matches(".template").to_string();
        }
        let target = dest.join(&target_name);
        let text = fs::read_to_string(&source_path)?;
        // La version du CLI est celle du SDK avec lequel il a été publié : un module scaffoldé
        // compile donc contre le SDK que cette commande connaît, et non contre des chemins
        // relatifs qui ne résolvent que dans un checkout du dépôt.
        let rendered = text
            .replace("{{MODULE_NAME}}", module_name)
            .replace("{{SDK_VERSION}}", env!("CARGO_PKG_VERSION"));
        fs::write(&target, rendered).with_context(|| format!("write {}", target.display()))?;
    }
    Ok(())
}
