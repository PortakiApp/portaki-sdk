//! Every i18n key a module uses exists in its `fr` and `en` bundles.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde_json::Value;

use super::invoke::Invocation;
use super::{emails, operations, surfaces, Findings, Module, MANIFEST_FILE};

/// Where a module keeps its locale bundles, one `<locale>.json` per locale.
pub const I18N_DIR: &str = "i18n";

/// The prefix a rendered string carries when the shell is to translate it.
pub const I18N_PREFIX: &str = "i18n:";

/// The locales every key must exist in, by language.
const REQUIRED_LANGUAGES: [&str; 2] = ["fr", "en"];

pub(super) fn check(module: &Module) -> Result<(), Findings> {
    Findings::of("i18n", problems(module))
}

fn problems(module: &Module) -> Vec<String> {
    let mut problems = Vec::new();
    let bundles = match read_bundles(&module.root().join(I18N_DIR)) {
        Ok(bundles) => bundles,
        Err(error) => return vec![error],
    };

    let mut required: Vec<(&str, BTreeSet<String>)> = Vec::new();
    for language in REQUIRED_LANGUAGES {
        match bundles
            .iter()
            .find(|(locale, _)| language_of(locale) == language)
        {
            Some((_, keys)) => {
                required.push((language, keys.clone()));
            }
            None => problems.push(format!(
                "no `{language}` bundle in {I18N_DIR}/ (e.g. {language}.json or \
                 {language}-XX.json) — the shells ship fr and en"
            )),
        }
    }

    if let Ok(Some(manifest)) = module.manifest() {
        let languages: BTreeSet<String> = bundles
            .iter()
            .map(|(locale, _)| language_of(locale))
            .collect();
        problems.extend(config_label_problems(&manifest, &languages));
    }

    let used = used_keys(module);
    for (key, origins) in &used {
        let missing: Vec<&str> = required
            .iter()
            .filter(|(_, keys)| !keys.contains(key))
            .map(|(language, _)| *language)
            .collect();
        if !missing.is_empty() {
            let origins: Vec<&str> = origins.iter().map(String::as_str).collect();
            let bundles = if missing.len() == 1 {
                "bundle"
            } else {
                "bundles"
            };
            problems.push(format!(
                "`{key}` is missing from the {} {bundles} — used by {}",
                missing.join(" and "),
                origins.join(", ")
            ));
        }
    }
    problems
}

/// Every `config.fields[]` label — and description, and option label, when there is one — has
/// a text in each language the module ships a bundle for: the dashboard shows them all.
fn config_label_problems(manifest: &Value, languages: &BTreeSet<String>) -> Vec<String> {
    let mut problems = Vec::new();
    let mut check = |what: String, text: Option<&Value>| {
        let given: BTreeSet<String> = text
            .and_then(Value::as_object)
            .into_iter()
            .flatten()
            .filter(|(_, text)| text.as_str().is_some_and(|text| !text.trim().is_empty()))
            .map(|(locale, _)| language_of(locale))
            .collect();
        let missing: Vec<&str> = languages
            .iter()
            .filter(|language| !given.contains(*language))
            .map(String::as_str)
            .collect();
        if !missing.is_empty() {
            problems.push(format!(
                "{MANIFEST_FILE} {what} has no {} text — declare its i18n key in every bundle",
                missing.join(" / ")
            ));
        }
    };
    for field in manifest["config"]["fields"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let key = field["key"].as_str().unwrap_or("?");
        check(format!("config field `{key}` label"), field.get("label"));
        if let Some(description) = field.get("description") {
            check(
                format!("config field `{key}` description"),
                Some(description),
            );
        }
        for option in field["options"].as_array().into_iter().flatten() {
            let value = option["value"].as_str().unwrap_or("?");
            check(
                format!("config field `{key}` option `{value}` label"),
                option.get("label"),
            );
        }
    }
    problems
}

/// Every key the module uses, with where it was seen.
fn used_keys(module: &Module) -> BTreeMap<String, BTreeSet<String>> {
    let mut used: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut note = |key: &str, origin: String| {
        let key = key.trim();
        if !key.is_empty() {
            used.entry(key.to_string()).or_default().insert(origin);
        }
    };

    if let Ok(Some(manifest)) = module.manifest() {
        for route in manifest
            .get("guestSurfaces")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(key) = route.get("labelKey").and_then(Value::as_str) {
                let surface = route
                    .get("surfaceId")
                    .and_then(Value::as_str)
                    .unwrap_or("?");
                note(key, format!("{MANIFEST_FILE} guestSurfaces `{surface}`"));
            }
        }
    }

    let mut from_invocation = |origin: String, invocation: &Invocation| {
        if let super::invoke::Outcome::Answered(tree) = &invocation.outcome {
            let mut prefixed = Vec::new();
            prefixed_strings(tree, &mut prefixed);
            for key in prefixed {
                note(&key, origin.clone());
            }
        }
        for key in &invocation.translated_keys {
            note(key, origin.clone());
        }
    };

    for (declaration, invocation) in surfaces::render_all(module) {
        from_invocation(super::invoke::describe(declaration), &invocation);
    }
    for (declaration, _, invocation) in operations::dispatch_all(module) {
        // An answer is data, not UI: only what the handler asked the host to translate counts.
        let translated_only = Invocation {
            outcome: super::invoke::Outcome::Failed(String::new()),
            translated_keys: invocation.translated_keys,
        };
        from_invocation(super::invoke::describe(declaration), &translated_only);
    }
    let (_, compositions) = emails::compose_all(module);
    for (label, invocation) in compositions {
        let translated_only = Invocation {
            outcome: super::invoke::Outcome::Failed(String::new()),
            translated_keys: invocation.translated_keys,
        };
        from_invocation(label, &translated_only);
    }
    used
}

/// `"i18n:host.title"` → `host.title`, anywhere in a JSON tree.
fn prefixed_strings(value: &Value, into: &mut Vec<String>) {
    match value {
        Value::String(text) => {
            if let Some(key) = text.strip_prefix(I18N_PREFIX) {
                into.push(key.to_string());
            }
        }
        Value::Array(items) => items.iter().for_each(|item| prefixed_strings(item, into)),
        Value::Object(map) => map.values().for_each(|item| prefixed_strings(item, into)),
        _ => {}
    }
}

/// `fr-FR` → `fr`.
fn language_of(locale: &str) -> String {
    locale
        .split(['-', '_'])
        .next()
        .unwrap_or(locale)
        .to_ascii_lowercase()
}

/// `(locale, keys)` for every `*.json` of `dir`, nested objects flattened with dots.
fn read_bundles(dir: &Path) -> Result<Vec<(String, BTreeSet<String>)>, String> {
    let entries = std::fs::read_dir(dir).map_err(|error| {
        format!(
            "cannot read {} ({error}) — a module ships its strings in {I18N_DIR}/fr-FR.json and \
             {I18N_DIR}/en-US.json",
            dir.display()
        )
    })?;

    let mut bundles = Vec::new();
    for entry in entries {
        let path = entry
            .map_err(|error| format!("cannot list {}: {error}", dir.display()))?
            .path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let Some(locale) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        let raw = std::fs::read_to_string(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        let bundle: Value = serde_json::from_str(&raw)
            .map_err(|error| format!("{} is not JSON: {error}", path.display()))?;
        let mut keys = BTreeSet::new();
        flatten(&bundle, "", &mut keys);
        bundles.push((locale.to_string(), keys));
    }
    bundles.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(bundles)
}

fn flatten(value: &Value, prefix: &str, into: &mut BTreeSet<String>) {
    match value {
        Value::Object(map) => {
            for (key, inner) in map {
                let path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                flatten(inner, &path, into);
            }
        }
        _ => {
            if !prefix.is_empty() {
                into.insert(prefix.to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_nested_bundle_reads_as_dotted_keys() {
        let mut keys = BTreeSet::new();
        flatten(
            &json!({ "host": { "title": "T", "section": { "help": "H" } }, "nav.x": "X" }),
            "",
            &mut keys,
        );
        assert_eq!(
            keys.into_iter().collect::<Vec<_>>(),
            vec!["host.section.help", "host.title", "nav.x"]
        );
    }

    #[test]
    fn config_labels_need_every_bundle_language() {
        let languages: BTreeSet<String> = ["en", "fr"].map(String::from).into();
        let manifest = json!({ "config": { "fields": [
            { "key": "ssid", "label": { "fr": "Réseau", "en-US": "Network" } },
            { "key": "password", "label": { "fr": "Mot de passe", "en": " " },
              "description": { "fr": "Au dos" } },
            { "key": "contacts" },
            { "key": "security", "label": { "fr": "S", "en": "S" },
              "options": [{ "value": "wep", "label": { "fr": "WEP" } }] },
        ] } });

        assert_eq!(
            config_label_problems(&manifest, &languages),
            vec![
                "portaki.module.json config field `password` label has no en text — declare its i18n key in every bundle",
                "portaki.module.json config field `password` description has no en text — declare its i18n key in every bundle",
                "portaki.module.json config field `contacts` label has no en / fr text — declare its i18n key in every bundle",
                "portaki.module.json config field `security` option `wep` label has no en text — declare its i18n key in every bundle",
            ]
        );
        assert!(config_label_problems(&json!({}), &languages).is_empty());
    }

    #[test]
    fn a_locale_names_its_language() {
        assert_eq!(language_of("fr-FR"), "fr");
        assert_eq!(language_of("en_US"), "en");
        assert_eq!(language_of("EN"), "en");
    }

    #[test]
    fn prefixed_strings_are_found_anywhere_in_a_tree() {
        let mut found = Vec::new();
        prefixed_strings(
            &json!({ "root": { "title": "i18n:a", "children": [{ "text": "i18n:b" }, "plain"] } }),
            &mut found,
        );
        found.sort();
        assert_eq!(found, vec!["a", "b"]);
    }
}
