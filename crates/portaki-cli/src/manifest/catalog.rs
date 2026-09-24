//! Les métadonnées de catalogue que le code sait déjà dire.
//!
//! `portaki.module.json` répétait l'id, la version, l'auteur, le nom et la description : autant
//! de choses que `portaki_module!(…)`, `Cargo.toml` et les bundles i18n portent déjà. Le build
//! les en tire et comble ce que le manifeste écrit à la main ne dit pas ; ce qu'il dit l'emporte,
//! le temps que les modules s'en délestent.

use std::path::Path;

use anyhow::{Context, Result};
use serde_json::{json, Map, Value};

use super::generator::EmissionFile;

/// Là où `portaki build` dépose le catalogue déduit.
pub const BUILT_CATALOG: &str = "target/portaki/catalog.json";

/// Le catalogue déduit de l'émission `module` et des bundles i18n.
///
/// Le nom et la description sont les traductions de leurs clés, par langue courte (`fr-FR` →
/// `fr`) ; une langue sans la clé est omise plutôt que remplie de la clé brute.
pub fn catalog_defaults(emissions: &[EmissionFile], i18n_dir: &Path, locales: &[String]) -> Value {
    let Some(module) = emissions.iter().find(|e| e.kind == "module") else {
        return Value::Object(Map::new());
    };
    let data = &module.data;
    let extra = data["catalog"].as_object().cloned().unwrap_or_default();

    let bundles: Vec<(String, Value)> = locales
        .iter()
        .filter_map(|locale| {
            let text = std::fs::read_to_string(i18n_dir.join(format!("{locale}.json"))).ok()?;
            let lang = locale.split('-').next().unwrap_or(locale).to_string();
            Some((lang, serde_json::from_str(&text).ok()?))
        })
        .collect();
    let translated = |key: &Value| -> Value {
        let key = key.as_str().unwrap_or_default();
        Value::Object(
            bundles
                .iter()
                .filter_map(|(lang, bundle)| Some((lang.clone(), bundle.get(key)?.clone())))
                .collect(),
        )
    };

    let mut author = json!({ "name": data["author"]["name"] });
    if let Some(url) = extra.get("authorUrl") {
        author["url"] = url.clone();
    }
    if let Some(kind) = extra.get("type") {
        author["type"] = kind.clone();
    }

    let mut catalog = json!({
        "id": data["id"],
        "version": data["version"],
        "name": translated(&data["displayName"]),
        "description": translated(&data["description"]),
        "author": author,
    });
    for key in ["icon", "type", "maturity", "sortOrder"] {
        if let Some(value) = extra.get(key) {
            catalog[key] = value.clone();
        }
    }
    catalog
}

/// Comble les clés que le manifeste écrit à la main ne porte pas. Ce qu'il porte l'emporte.
pub fn fill_catalog(raw: &str, catalog: &str) -> Result<String> {
    let catalog: Value = serde_json::from_str(catalog).context("parse built catalog")?;
    let Some(defaults) = catalog.as_object().filter(|c| !c.is_empty()) else {
        return Ok(raw.to_string());
    };
    let mut manifest: Value = serde_json::from_str(raw).context("parse module manifest")?;
    let Some(object) = manifest.as_object_mut() else {
        return Ok(raw.to_string());
    };
    let missing: Vec<_> = defaults
        .iter()
        .filter(|(key, value)| !object.contains_key(*key) && !is_empty(value))
        .collect();
    if missing.is_empty() {
        return Ok(raw.to_string());
    }
    for (key, value) in missing {
        object.insert(key.clone(), value.clone());
    }
    serde_json::to_string_pretty(&manifest).context("serialise module manifest")
}

/// Un nom sans aucune traduction n'apprend rien au catalogue.
fn is_empty(value: &Value) -> bool {
    value.is_null() || value.as_object().is_some_and(Map::is_empty)
}

#[cfg(test)]
mod tests {
    use super::{catalog_defaults, fill_catalog};
    use crate::manifest::generator::EmissionFile;
    use serde_json::json;

    fn module() -> Vec<EmissionFile> {
        vec![EmissionFile {
            kind: "module".into(),
            data: json!({
                "id": "issue-report",
                "version": "0.6.0",
                "displayName": "module.displayName",
                "description": "module.description",
                "author": { "name": "Portaki" },
                "catalog": { "icon": "danger-triangle", "type": "official",
                             "authorUrl": "https://portaki.app", "sortOrder": 90 },
            }),
        }]
    }

    #[test]
    fn the_catalog_reads_the_module_and_its_translations() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("fr-FR.json"),
            r#"{"module.displayName":"Signaler","module.description":"Un souci"}"#,
        )
        .expect("fr");
        std::fs::write(
            dir.path().join("en-US.json"),
            r#"{"module.displayName":"Report"}"#,
        )
        .expect("en");

        let catalog = catalog_defaults(
            &module(),
            dir.path(),
            &["en-US".to_string(), "fr-FR".to_string()],
        );

        assert_eq!(
            catalog,
            json!({
                "id": "issue-report",
                "version": "0.6.0",
                "name": { "en": "Report", "fr": "Signaler" },
                "description": { "fr": "Un souci" },
                "author": { "name": "Portaki", "url": "https://portaki.app", "type": "official" },
                "icon": "danger-triangle",
                "type": "official",
                "sortOrder": 90,
            })
        );
    }

    #[test]
    fn the_hand_written_manifest_keeps_what_it_says() {
        let raw = r#"{"id":"issue-report","icon":"bell","permissions":["repo"]}"#;
        let catalog = r#"{"id":"x","icon":"danger-triangle","maturity":"stable","name":{}}"#;

        let filled: serde_json::Value =
            serde_json::from_str(&fill_catalog(raw, catalog).expect("fill")).expect("parse");

        assert_eq!(filled["id"], "issue-report");
        assert_eq!(filled["icon"], "bell");
        assert_eq!(filled["maturity"], "stable");
        assert!(
            filled.get("name").is_none(),
            "an untranslated name is left out"
        );
    }

    #[test]
    fn nothing_to_fill_leaves_the_manifest_untouched() {
        let raw = r#"{"id":"issue-report"}"#;
        assert_eq!(fill_catalog(raw, r#"{"id":"x"}"#).expect("fill"), raw);
        assert_eq!(fill_catalog(raw, "{}").expect("fill"), raw);
    }
}
