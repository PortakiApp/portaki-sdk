//! `portaki add permission|connector|language` — declare what the module uses, in the place
//! where the SDK reads it.
//!
//! - a permission is a `portaki-sdk` feature in `Cargo.toml` ([`crate::commands::permissions`]);
//! - a built-in connector, the `#[portaki_sdk::connector(builtin = "…")]` attribute in the code —
//!   `portaki build` derives `connectors:<id>` from it;
//! - a language, an `i18n/<locale>.json` bundle (and `email_i18n/`) with the same keys and empty
//!   texts: `portaki check --only i18n` then lists what is left to write.

use std::path::Path;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde_json::{Map, Value};

use crate::commands::{i18n, permissions};
use crate::{tr, ui, workspace};

/// The built-in connectors, as `portaki-connectors` serves them (one submodule each).
pub const BUILTIN_CONNECTORS: [&str; 7] = [
    "open-weather",
    "open-agenda",
    "google-places",
    "mapbox",
    "osm-nominatim",
    "nuki",
    "tiqets",
];

#[derive(Debug, Parser)]
/// Arguments for `portaki add`.
pub struct AddArgs {
    #[command(subcommand)]
    pub what: AddCommand,
}

#[derive(Debug, Subcommand)]
pub enum AddCommand {
    /// Declare a permission: turns on the portaki-sdk feature that grants it.
    Permission(permissions::AddArgs),
    /// Declare a built-in connector: writes its `#[portaki_sdk::connector]` in src/lib.rs.
    Connector(ConnectorArgs),
    /// Add a language: a bundle with every key of the others, texts to write.
    Language(LanguageArgs),
}

#[derive(Debug, Parser)]
/// Arguments for `portaki add connector`.
pub struct ConnectorArgs {
    /// The built-in connector id, e.g. `open-weather`.
    pub id: String,
    #[command(flatten)]
    pub modules: workspace::ModuleArgs,
}

#[derive(Debug, Parser)]
/// Arguments for `portaki add language`.
pub struct LanguageArgs {
    /// The language, `de` or a locale `de-DE`.
    pub lang: String,
    #[command(flatten)]
    pub modules: workspace::ModuleArgs,
}

/// Runs `portaki add`.
pub fn run(args: AddArgs) -> Result<()> {
    match args.what {
        AddCommand::Permission(args) => permissions::add(args),
        AddCommand::Connector(args) => connector(args),
        AddCommand::Language(args) => language(args),
    }
}

fn connector(args: ConnectorArgs) -> Result<()> {
    ui::header(
        "portaki add connector",
        &tr!(
            "Declare a built-in connector in the code — the build derives its permission.",
            "Déclarer un connecteur intégré dans le code — le build en déduit la permission."
        ),
    );
    if !BUILTIN_CONNECTORS.contains(&args.id.as_str()) {
        return Err(crate::exit::usage(tr!(
            "{} is not a built-in connector — known: {}; your own API is declared with #[portaki_sdk::custom_connector]",
            "{} n'est pas un connecteur intégré — connus : {} ; votre propre API se déclare avec #[portaki_sdk::custom_connector]",
            args.id,
            BUILTIN_CONNECTORS.join(", ")
        )));
    }
    let mut changed = false;
    for member in args.modules.resolve()? {
        if declares(&member.root, &args.id) {
            ui::skipped(tr!(
                "{} already declares {}",
                "{} déclare déjà {}",
                member.id,
                args.id
            ));
            continue;
        }
        let lib = member.root.join("src/lib.rs");
        let mut source =
            std::fs::read_to_string(&lib).with_context(|| format!("read {}", lib.display()))?;
        source.push_str(&declaration(&args.id));
        std::fs::write(&lib, source).with_context(|| format!("write {}", lib.display()))?;
        ui::success(tr!(
            "{} declares the connector {} (src/lib.rs)",
            "{} déclare le connecteur {} (src/lib.rs)",
            member.id,
            args.id
        ));
        changed = true;
    }
    if changed {
        ui::next(&[
            (
                "cargo add portaki-connectors",
                &tr!(
                    "typed clients to call it (skip if already there)",
                    "les clients typés pour l'appeler (inutile s'il y est déjà)"
                ),
            ),
            (
                "portaki connectors",
                &tr!(
                    "what it needs to actually call — a credential bound for the workspace",
                    "ce qu'il lui faut pour appeler — un identifiant lié à l'espace de l'hôte"
                ),
            ),
        ]);
    } else {
        crate::exit::nothing_to_do();
    }
    ui::blank();
    Ok(())
}

/// Does the module already declare this connector, in any file under `src/`?
fn declares(module_root: &Path, id: &str) -> bool {
    let needle = format!("builtin = \"{id}\"");
    walkdir::WalkDir::new(module_root.join("src"))
        .into_iter()
        .flatten()
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "rs"))
        .any(|entry| {
            std::fs::read_to_string(entry.path()).is_ok_and(|source| source.contains(&needle))
        })
}

/// `open-weather` → the declaration, under a `UsesOpenWeather` type.
fn declaration(id: &str) -> String {
    let name: String = id
        .split('-')
        .map(|word| {
            let mut chars = word.chars();
            chars
                .next()
                .map(|first| first.to_ascii_uppercase().to_string() + chars.as_str())
                .unwrap_or_default()
        })
        .collect();
    format!(
        "\n/// The built-in `{id}` connector — `portaki build` declares `connectors:{id}`.\n\
         #[portaki_sdk::connector(builtin = \"{id}\")]\n\
         pub struct Uses{name};\n"
    )
}

fn language(args: LanguageArgs) -> Result<()> {
    ui::header(
        "portaki add language",
        &tr!(
            "Add a language: every key of the other bundles, texts to write.",
            "Ajouter une langue : chaque clé des autres bundles, textes à écrire."
        ),
    );
    let lang = args.lang.trim().to_string();
    let valid = lang.len() >= 2
        && lang
            .split('-')
            .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_alphabetic()));
    if !valid {
        return Err(crate::exit::usage(tr!(
            "{lang} is not a language — expected `de` or `de-DE`",
            "{lang} n'est pas une langue — attendu `de` ou `de-DE`"
        )));
    }
    let mut changed = false;
    let mut short = lang.clone();
    for member in args.modules.resolve()? {
        for dir in i18n::BUNDLE_DIRS {
            let bundles = i18n::read_bundles(&member.root.join(dir))?;
            let Some(written) = add_bundle(&member.root.join(dir), &bundles, &lang)? else {
                continue;
            };
            short = written.split('-').next().unwrap_or(&lang).to_string();
            ui::wrote(dir, format!("{dir}/{written}.json"));
            changed = true;
        }
    }
    if !changed {
        ui::skipped(tr!(
            "{lang} is already there, or the module has no bundle to copy the keys from",
            "{lang} est déjà là, ou le module n'a aucun bundle d'où copier les clés"
        ));
        crate::exit::nothing_to_do();
        ui::blank();
        return Ok(());
    }
    ui::advice(tr!(
        "once its texts are written, add \"{short}\" to publishedLangs in listing.json — every stable version then needs a changelog line in it",
        "une fois ses textes écrits, ajoutez « {short} » à publishedLangs dans listing.json — chaque version stable demandera alors une ligne de changelog dans cette langue"
    ));
    ui::next(&[(
        "portaki check --only i18n",
        &tr!("the texts left to write", "les textes qu'il reste à écrire"),
    )]);
    ui::blank();
    Ok(())
}

/// Writes `lang`'s bundle into `dir`, with the keys of all the others and empty texts. The file
/// name follows the ones already there: `de-DE.json` next to `fr-FR.json`, `de.json` next to
/// `fr.json`. `None`: nothing to copy, or the language is already there.
fn add_bundle(
    dir: &Path,
    bundles: &std::collections::BTreeMap<String, Map<String, Value>>,
    lang: &str,
) -> Result<Option<String>> {
    if bundles.is_empty() {
        return Ok(None);
    }
    let regional = bundles.keys().any(|locale| locale.contains('-'));
    let locale = match (lang.split_once('-'), regional) {
        (Some(_), _) | (None, false) => lang.to_string(),
        (None, true) => match lang {
            "en" => "en-US".to_string(),
            other => format!("{other}-{}", other.to_ascii_uppercase()),
        },
    };
    let prefix = locale
        .split('-')
        .next()
        .unwrap_or(&locale)
        .to_ascii_lowercase();
    if bundles.keys().any(|existing| {
        existing
            .split('-')
            .next()
            .unwrap_or(existing)
            .eq_ignore_ascii_case(&prefix)
    }) {
        return Ok(None);
    }
    let keys: Map<String, Value> = bundles
        .values()
        .flat_map(Map::keys)
        .map(|key| (key.clone(), Value::String(String::new())))
        .collect();
    let path = dir.join(format!("{locale}.json"));
    std::fs::write(&path, serde_json::to_string_pretty(&keys)? + "\n")
        .with_context(|| format!("write {}", path.display()))?;
    Ok(Some(locale))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The list follows `portaki-connectors`: a connector that goes in there must go in here.
    #[test]
    fn the_builtin_list_is_the_connectors_crate_table() {
        let table = include_str!("../../../portaki-connectors/src/lib.rs");
        let listed: Vec<&str> = table
            .lines()
            .filter_map(|line| line.strip_prefix("//! | [`"))
            .filter_map(|row| row.split('`').nth(2))
            .collect();
        assert_eq!(listed, BUILTIN_CONNECTORS.to_vec());
    }

    #[test]
    fn a_declaration_names_its_type_after_the_connector() {
        let written = declaration("open-weather");
        assert!(written.contains("#[portaki_sdk::connector(builtin = \"open-weather\")]"));
        assert!(written.contains("pub struct UsesOpenWeather;"));
    }

    #[test]
    fn a_new_bundle_carries_every_key_and_follows_the_naming() {
        let dir = tempfile::tempdir().unwrap();
        let mut bundles = std::collections::BTreeMap::new();
        bundles.insert(
            "fr-FR".to_string(),
            serde_json::json!({ "a": "x", "b": "y" })
                .as_object()
                .unwrap()
                .clone(),
        );
        bundles.insert(
            "en-US".to_string(),
            serde_json::json!({ "a": "x", "c": "z" })
                .as_object()
                .unwrap()
                .clone(),
        );

        assert_eq!(
            add_bundle(dir.path(), &bundles, "de").unwrap().as_deref(),
            Some("de-DE")
        );
        let written: Value =
            serde_json::from_str(&std::fs::read_to_string(dir.path().join("de-DE.json")).unwrap())
                .unwrap();
        assert_eq!(written, serde_json::json!({ "a": "", "b": "", "c": "" }));

        assert!(add_bundle(dir.path(), &bundles, "fr").unwrap().is_none());
        assert!(add_bundle(dir.path(), &Default::default(), "de")
            .unwrap()
            .is_none());
    }
}
