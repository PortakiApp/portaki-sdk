//! `portaki i18n check` — every text in every language the module ships.
//!
//! A key present in `fr-FR.json` and absent from `en-US.json` shows the raw key to an English
//! guest; the platform reports it as `i18n_incomplete`. This compares the bundles with each
//! other, in `i18n/` and in `email_i18n/`: whatever one language says, every other must say too.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde_json::{Map, Value};

use crate::{ui, workspace};

/// The bundle directories a module may keep.
pub(crate) const BUNDLE_DIRS: [&str; 2] = ["i18n", "email_i18n"];

#[derive(Debug, Parser)]
/// Arguments for `portaki i18n`.
pub struct I18nArgs {
    #[command(subcommand)]
    pub command: I18nCommand,
}

#[derive(Debug, Subcommand)]
pub enum I18nCommand {
    /// Fail on a key missing or empty in one language of the bundles.
    Check(CheckArgs),
}

#[derive(Debug, Parser)]
/// Arguments for `portaki i18n check`.
pub struct CheckArgs {
    /// In a repository holding several modules, the one to check.
    #[arg(long, conflicts_with = "all")]
    pub module: Option<String>,
    /// Check every module of the repository.
    #[arg(long)]
    pub all: bool,
}

/// Runs `portaki i18n` — the former name of `portaki check --only i18n`.
pub fn run(args: I18nArgs) -> Result<()> {
    let I18nCommand::Check(args) = args.command;
    ui::warn(crate::tr!(
        "portaki i18n check is now portaki check --only i18n — use that from now on",
        "portaki i18n check devient portaki check --only i18n — utilisez désormais ce nom"
    ));
    for member in workspace::resolve(args.module.as_deref(), Some(args.all))? {
        gate(&member.root).with_context(|| format!("module {}", member.id))?;
    }
    ui::blank();
    Ok(())
}

/// Le contrôle `i18n` de la porte : chaque texte, dans chaque langue des bundles.
pub fn gate(module_root: &Path) -> Result<()> {
    let checking = ui::step(crate::tr!(
        "checking every text in every language",
        "vérification de chaque texte dans chaque langue"
    ));
    let problems = problems(module_root)?;
    if problems.is_empty() {
        checking.done(crate::tr!(
            "every text, in every language",
            "chaque texte, dans chaque langue"
        ));
        return Ok(());
    }
    checking.abandon();
    for problem in &problems {
        ui::detail(problem);
    }
    anyhow::bail!(crate::tr!(
        "{} text(s) missing — write them, then run portaki check --only i18n again",
        "{} texte(s) manquant(s) — écrivez-les, puis relancez portaki check --only i18n",
        problems.len()
    ))
}

/// Ce qui manque, bundle par bundle, dans `i18n/` et `email_i18n/`.
pub fn problems(module_root: &Path) -> Result<Vec<String>> {
    let mut problems = Vec::new();
    for dir in BUNDLE_DIRS {
        let bundles = read_bundles(&module_root.join(dir))?;
        problems.extend(
            incomplete(&bundles)
                .into_iter()
                .map(|problem| format!("{dir}/{problem}")),
        );
    }
    Ok(problems)
}

/// `<locale>.json` → its keys and texts. Empty when the directory does not exist.
pub(crate) fn read_bundles(dir: &Path) -> Result<BTreeMap<String, Map<String, Value>>> {
    let mut bundles = BTreeMap::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(bundles);
    };
    for path in entries.flatten().map(|entry| entry.path()) {
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let raw =
            std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        let texts: Map<String, Value> = serde_json::from_str(&raw)
            .with_context(|| format!("{} is not a JSON object", path.display()))?;
        let locale = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        bundles.insert(locale, texts);
    }
    Ok(bundles)
}

/// `<locale>.json: `key` missing` for every key another bundle has, and `… empty` for a blank text.
fn incomplete(bundles: &BTreeMap<String, Map<String, Value>>) -> Vec<String> {
    let every: BTreeSet<&String> = bundles.values().flat_map(Map::keys).collect();
    let mut problems = Vec::new();
    for (locale, texts) in bundles {
        for key in &every {
            match texts.get(*key) {
                None => problems.push(format!("{locale}.json: `{key}` missing")),
                Some(Value::String(text)) if text.trim().is_empty() => {
                    problems.push(format!("{locale}.json: `{key}` empty"))
                }
                _ => {}
            }
        }
    }
    problems
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bundles(raw: &[(&str, Value)]) -> BTreeMap<String, Map<String, Value>> {
        raw.iter()
            .map(|(locale, texts)| (locale.to_string(), texts.as_object().unwrap().clone()))
            .collect()
    }

    #[test]
    fn a_key_one_language_lacks_is_named() {
        let found = incomplete(&bundles(&[
            (
                "fr-FR",
                serde_json::json!({ "title": "Accès", "cta": "Voir" }),
            ),
            (
                "en-US",
                serde_json::json!({ "title": "Access", "extra": " " }),
            ),
        ]));

        assert_eq!(
            found,
            vec![
                "en-US.json: `cta` missing",
                "en-US.json: `extra` empty",
                "fr-FR.json: `extra` missing",
            ]
        );
    }

    #[test]
    fn matching_bundles_pass() {
        let found = incomplete(&bundles(&[
            ("fr", serde_json::json!({ "a": "x" })),
            ("en", serde_json::json!({ "a": "y" })),
        ]));

        assert!(found.is_empty());
    }
}
