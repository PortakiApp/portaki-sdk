//! The module's manifest — hand-written or built — against the schema the registry validates with,
//! and against the translated text its `#[portaki_sdk::config]` declares.

use serde_json::Value;

use super::{Findings, Module, BUILT_MANIFEST_FILE, MANIFEST_FILE};

/// `schema/module.v1.json` of the SDK, carried in this crate so a check never needs the network.
///
/// A copy, because a published crate cannot reach outside its own directory; a test of this crate
/// fails when it drifts from the repository's `schema/module.v1.json`.
pub const MODULE_SCHEMA_V1: &str = include_str!("../../schema/module.v1.json");

pub(super) fn check(module: &Module) -> Result<(), Findings> {
    Findings::of("manifest", problems(module))
}

fn problems(module: &Module) -> Vec<String> {
    let manifest = match module.manifest() {
        Ok(Some(manifest)) => manifest,
        // Nothing built yet: the manifest is written from the code by `portaki build`, and
        // `portaki lint` validates it right after — `cargo test` alone has nothing to read.
        Ok(None) => return Vec::new(),
        Err(error) => return vec![error],
    };

    let schema: serde_json::Value =
        serde_json::from_str(MODULE_SCHEMA_V1).expect("the bundled module.v1.json is JSON");
    let validator = match jsonschema::validator_for(&schema) {
        Ok(validator) => validator,
        Err(error) => {
            return vec![format!(
                "the bundled module.v1.json does not compile: {error}"
            )]
        }
    };

    let file = if module.root().join(BUILT_MANIFEST_FILE).exists() {
        BUILT_MANIFEST_FILE
    } else {
        MANIFEST_FILE
    };
    let mut problems: Vec<String> = validator
        .iter_errors(&manifest)
        .map(|error| {
            let at = error.instance_path().to_string();
            let at = if at.is_empty() { "/".to_string() } else { at };
            format!("{file} at {at}: {error}")
        })
        .collect();
    let code: Vec<Value> = portaki_sdk::config::declarations()
        .filter_map(|declaration| serde_json::from_str::<Vec<Value>>(declaration.fields).ok())
        .flatten()
        .collect();
    problems.extend(
        config_problems(&manifest, &code, portaki_sdk::wasm::registry::params_shape)
            .into_iter()
            .map(|problem| format!("{file}: {problem}")),
    );
    problems
}

/// The translated text the code declares is what the manifest says — the platform writes a save
/// by the manifest, and a `localized` field or sub-key it does not know is overwritten whole, every
/// other language lost. `item.localized` and `item.id` name real fields of the row type.
///
/// `code` is `#[portaki_sdk::config]`'s fields as emitted (`itemType` unresolved); `shape_of` the
/// `#[params]` shape of a row type.
fn config_problems(
    manifest: &Value,
    code: &[Value],
    shape_of: impl Fn(&str) -> Option<Value>,
) -> Vec<String> {
    let mut problems = Vec::new();
    let declared = manifest["config"]["fields"].as_array();
    for field in code {
        let key = field["key"].as_str().unwrap_or("?");
        let Some(stated) = declared
            .into_iter()
            .flatten()
            .find(|stated| stated["key"] == key)
        else {
            continue;
        };
        let mut resolved = [field.clone()];
        portaki_sdk::config::resolve_items(&mut resolved, &shape_of);
        let [resolved] = resolved;

        if resolved["type"] == "localized" && stated["type"] != "localized" {
            problems.push(format!(
                "config field `{key}` is an I18nText in the code but not `localized` — \
                 rebuild with portaki build"
            ));
        }
        let code_localized = &resolved["item"]["localized"];
        if code_localized
            .as_array()
            .is_some_and(|keys| !keys.is_empty())
            && &stated["item"]["localized"] != code_localized
        {
            problems.push(format!(
                "config field `{key}` item.localized must be {code_localized} (the row's I18nText \
                 fields) — rebuild with portaki build"
            ));
        }

        let Some(row) = field["itemType"].as_str() else {
            continue;
        };
        let Some(shape) = shape_of(row) else {
            continue;
        };
        let names: Vec<&str> = shape["fields"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|f| f["name"].as_str())
            .collect();
        let named = stated["item"]["localized"]
            .as_array()
            .into_iter()
            .flatten()
            .chain(stated["item"].get("id"))
            .filter_map(Value::as_str);
        for sub_key in named {
            if !names.contains(&sub_key) {
                problems.push(format!(
                    "config field `{key}` item names `{sub_key}`, which is not a field of \
                     {row} ({})",
                    names.join(", ")
                ));
            }
        }
    }
    problems
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn step() -> Option<Value> {
        Some(json!({ "fields": [
            { "name": "id", "type": "string" },
            { "name": "title", "type": "ref", "ref": "I18nText" },
            { "name": "note", "type": "string" },
        ] }))
    }

    #[test]
    fn the_manifest_says_what_the_code_translates() {
        let code = [
            json!({ "key": "welcome", "type": "localized" }),
            json!({ "key": "steps", "type": "structured", "itemType": "Step" }),
            json!({ "key": "spots", "type": "structured", "itemType": "Step", "item": { "id": "slug" } }),
        ];
        let shape_of = |name: &str| if name == "Step" { step() } else { None };

        let built = json!({ "config": { "fields": [
            { "key": "welcome", "type": "localized" },
            { "key": "steps", "type": "structured", "item": { "id": "id", "localized": ["title"] } },
        ] } });
        assert!(config_problems(&built, &code, shape_of).is_empty());

        let stale = json!({ "config": { "fields": [
            { "key": "welcome", "type": "text" },
            { "key": "steps", "type": "structured", "item": { "localized": ["titel"] } },
            { "key": "spots", "type": "structured", "item": { "id": "slug", "localized": ["title"] } },
        ] } });
        assert_eq!(
            config_problems(&stale, &code, shape_of),
            vec![
                "config field `welcome` is an I18nText in the code but not `localized` — rebuild with portaki build",
                "config field `steps` item.localized must be [\"title\"] (the row's I18nText fields) — rebuild with portaki build",
                "config field `steps` item names `titel`, which is not a field of Step (id, title, note)",
                "config field `spots` item names `slug`, which is not a field of Step (id, title, note)",
            ]
        );
    }
}
