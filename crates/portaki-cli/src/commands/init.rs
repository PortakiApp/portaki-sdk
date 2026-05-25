//! `portaki init` — scaffold a module from templates.

use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::{Parser, ValueEnum};

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
            "template not found: {} (run from portaki-sdk-rust checkout)",
            template_dir.display()
        );
    }

    copy_template(&template_dir, &dest, &args.name)?;
    println!("Created module at {}", dest.display());
    println!("Next: cd {} && portaki build", dest.display());
    Ok(())
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
        let rendered = text.replace("{{MODULE_NAME}}", module_name);
        fs::write(&target, rendered).with_context(|| format!("write {}", target.display()))?;
    }
    Ok(())
}
