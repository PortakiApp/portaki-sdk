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
    problems.extend(
        shares_problems(&manifest)
            .into_iter()
            .chain(contract_problems(&manifest))
            .chain(core_problems(&manifest))
            .map(|problem| format!("{file}: {problem}")),
    );
    problems
}

/// What a provided capability hands its consumers must be the module's own: `path` one of its
/// guest surface paths (a consumer navigates there), `commands` some of its commands (the
/// platform lets a dependent module call those, and only those).
fn contract_problems(manifest: &Value) -> Vec<String> {
    let names = |key: &str, field: &str| -> Vec<String> {
        manifest[key]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|entry| entry[field].as_str().map(str::to_string))
            .collect()
    };
    let paths = names("guestSurfaces", "path");
    let commands = names("commands", "name");
    let mut problems = Vec::new();
    for entry in manifest["capabilities"]["provided"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let capability = entry["id"].as_str().unwrap_or("?");
        if let Some(path) = entry["path"].as_str() {
            if !paths.iter().any(|own| own == path) {
                problems.push(format!(
                    "capability `{capability}` opens at `{path}`, which is not one of the \
                     module's guest surface paths"
                ));
            }
        }
        for command in entry["commands"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            if !commands.iter().any(|own| own == command) {
                problems.push(format!(
                    "capability `{capability}` lets peers call `{command}`, which is not one of \
                     the module's commands"
                ));
            }
        }
    }
    problems
}

/// A core module is on every property with no switch: one that shows nothing anywhere is a
/// module nobody can see nor turn off.
fn core_problems(manifest: &Value) -> Option<String> {
    let shows = |key: &str| manifest[key].as_array().is_some_and(|s| !s.is_empty());
    (manifest["core"] == true && !shows("hostSurfaces") && !shows("guestSurfaces")).then(|| {
        "a `core` module declares no host nor guest surface — it would be installed everywhere \
         and seen nowhere"
            .to_string()
    })
}

/// What a provided capability `shares` is read by other modules: it must be one of the module's
/// own settings, and never a secret — the platform serves it to any peer, decrypted or not.
fn shares_problems(manifest: &Value) -> Vec<String> {
    let fields = manifest["config"]["fields"].as_array();
    let mut problems = Vec::new();
    for entry in manifest["capabilities"]["provided"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let capability = entry["id"].as_str().unwrap_or("?");
        for key in entry["shares"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            let field = fields
                .into_iter()
                .flatten()
                .find(|field| field["key"] == key);
            let problem = match field {
                None => "is not one of the module's config fields",
                Some(field) if field["type"] == "secret" => "is a secret field",
                Some(field)
                    if field["item"]["secret"]
                        .as_array()
                        .is_some_and(|keys| !keys.is_empty()) =>
                {
                    "has secret sub-keys (item.secret)"
                }
                Some(_) => continue,
            };
            problems.push(format!(
                "capability `{capability}` shares `{key}`, which {problem} — only the module's \
                 own non-secret settings can be shared"
            ));
        }
    }
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

        // A secret sub-key the manifest does not name is stored in clear, and a masked value sent
        // back by the form overwrites it.
        if let Some(code_secret) = resolved["item"].get("secret") {
            if &stated["item"]["secret"] != code_secret {
                problems.push(format!(
                    "config field `{key}` item.secret must be {code_secret} (the row's \
                     #[field(secret)] fields) — rebuild with portaki build"
                ));
            }
        }

        // What the battery accepts and the reviewer reads is the manifest: it says what the code
        // reveals, no more, no less.
        if resolved.get("reveal") != stated.get("reveal") {
            let code_reveal = resolved.get("reveal").cloned().unwrap_or(Value::Null);
            problems.push(format!(
                "config field `{key}` reveal must be {code_reveal} (its #[field(reveal(…))]) — \
                 rebuild with portaki build"
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
            .chain(stated["item"]["secret"].as_array().into_iter().flatten())
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

    #[test]
    fn a_capability_shares_its_own_plain_settings() {
        let manifest = json!({
            "config": { "fields": [
                { "key": "reveal_policy", "type": "select" },
                { "key": "reveal_hours", "type": "number" }
            ] },
            "capabilities": { "provided": [
                "access.smart_lock",
                { "id": "amenities.provide", "shares": ["reveal_policy", "reveal_hours"] }
            ] }
        });
        assert!(shares_problems(&manifest).is_empty());
    }

    #[test]
    fn a_capability_never_shares_a_secret_or_an_unknown_key() {
        let manifest = json!({
            "config": { "fields": [
                { "key": "door_code", "type": "secret" },
                { "key": "spots", "type": "structured", "item": { "secret": ["code"] } }
            ] },
            "capabilities": { "provided": [
                { "id": "amenities.provide", "shares": ["door_code", "spots", "nowhere"] }
            ] }
        });
        assert_eq!(
            shares_problems(&manifest),
            vec![
                "capability `amenities.provide` shares `door_code`, which is a secret field — \
                 only the module's own non-secret settings can be shared",
                "capability `amenities.provide` shares `spots`, which has secret sub-keys \
                 (item.secret) — only the module's own non-secret settings can be shared",
                "capability `amenities.provide` shares `nowhere`, which is not one of the \
                 module's config fields — only the module's own non-secret settings can be shared",
            ]
        );
    }

    #[test]
    fn a_capability_opens_its_own_path_and_lends_its_own_commands() {
        let manifest = json!({
            "guestSurfaces": [{ "surfaceId": "guest.form", "path": "provider/form" }],
            "commands": [{ "name": "submit" }],
            "capabilities": { "provided": [
                { "id": "report.issue", "path": "provider/form", "commands": ["submit"] },
                { "id": "other.thing", "path": "elsewhere", "commands": ["drop"] }
            ] }
        });
        assert_eq!(
            contract_problems(&manifest),
            vec![
                "capability `other.thing` opens at `elsewhere`, which is not one of the module's \
                 guest surface paths",
                "capability `other.thing` lets peers call `drop`, which is not one of the \
                 module's commands",
            ]
        );
    }

    #[test]
    fn the_schema_accepts_provided_capabilities_with_shares() {
        let schema: Value = serde_json::from_str(MODULE_SCHEMA_V1).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        let manifest = json!({
            "id": "provider", "name": { "fr": "P" }, "description": { "fr": "P" },
            "version": "1.0.0", "author": { "name": "A", "type": "official" }, "icon": "i", "type": "guest",
            "capabilities": { "provided": [
                "access.smart_lock",
                { "id": "amenities.provide", "shares": ["reveal_policy"] },
                { "id": "report.issue", "path": "provider/form", "commands": ["submit"] }
            ], "depends": [{ "id": "other.thing", "optional": true }] }
        });
        let errors: Vec<String> = validator
            .iter_errors(&manifest)
            .map(|e| e.to_string())
            .collect();
        assert!(errors.is_empty(), "{errors:?}");
        let mut wrong = manifest.clone();
        wrong["capabilities"]["provided"][1]["shares"] = json!("reveal_policy");
        assert!(!validator.is_valid(&wrong));
    }

    #[test]
    fn the_schema_accepts_a_core_module_on_the_listing_card() {
        let schema: Value = serde_json::from_str(MODULE_SCHEMA_V1).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        let manifest = json!({
            "id": "sections", "name": { "fr": "S" }, "description": { "fr": "S" },
            "version": "1.0.0", "author": { "name": "A", "type": "official" }, "icon": "i", "type": "guest",
            "core": true,
            "hostSurfaces": [{ "type": "property-listing-card", "pathSegment": "sections",
                               "label": { "fr": "Message d'accueil" } }]
        });
        let errors: Vec<String> = validator
            .iter_errors(&manifest)
            .map(|e| e.to_string())
            .collect();
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(core_problems(&manifest), None);

        let mut wrong = manifest.clone();
        wrong["core"] = json!("yes");
        assert!(!validator.is_valid(&wrong));

        let mut hidden = manifest.clone();
        hidden["hostSurfaces"] = json!([]);
        assert!(core_problems(&hidden).is_some());
        hidden["core"] = json!(false);
        assert_eq!(core_problems(&hidden), None);
    }

    fn step() -> Option<Value> {
        Some(json!({ "fields": [
            { "name": "id", "type": "string" },
            { "name": "title", "type": "ref", "ref": "I18nText" },
            { "name": "note", "type": "string" },
            { "name": "code", "type": "string", "secret": true },
        ] }))
    }

    #[test]
    fn the_manifest_says_what_the_code_translates() {
        let code = [
            json!({ "key": "welcome", "type": "localized" }),
            json!({ "key": "steps", "type": "structured", "itemType": "Step" }),
            json!({ "key": "spots", "type": "structured", "itemType": "Step", "item": { "id": "slug" } }),
            json!({ "key": "code", "type": "secret", "reveal": ["guest_stay"] }),
        ];
        let shape_of = |name: &str| if name == "Step" { step() } else { None };

        let built = json!({ "config": { "fields": [
            { "key": "welcome", "type": "localized" },
            { "key": "steps", "type": "structured", "item": { "id": "id", "localized": ["title"], "secret": ["code"] } },
            { "key": "code", "type": "secret", "reveal": ["guest_stay"] },
        ] } });
        assert!(config_problems(&built, &code, shape_of).is_empty());

        let stale = json!({ "config": { "fields": [
            { "key": "welcome", "type": "text" },
            { "key": "steps", "type": "structured", "item": { "localized": ["titel"] } },
            { "key": "spots", "type": "structured", "item": { "id": "slug", "localized": ["title"], "secret": ["code"] } },
            { "key": "code", "type": "secret", "reveal": ["guest_stay", "arrival_email"] },
        ] } });
        assert_eq!(
            config_problems(&stale, &code, shape_of),
            vec![
                "config field `welcome` is an I18nText in the code but not `localized` — rebuild with portaki build",
                "config field `steps` item.localized must be [\"title\"] (the row's I18nText fields) — rebuild with portaki build",
                "config field `steps` item.secret must be [\"code\"] (the row's #[field(secret)] fields) — rebuild with portaki build",
                "config field `steps` item names `titel`, which is not a field of Step (id, title, note, code)",
                "config field `spots` item names `slug`, which is not a field of Step (id, title, note, code)",
                "config field `code` reveal must be [\"guest_stay\"] (its #[field(reveal(…))]) — rebuild with portaki build",
            ]
        );
    }

    /// Since portaki-sdk 9.3 the published manifest carries `connectors` (ADR-0021). A real one —
    /// `weather`, built by the 9.4 CLI — must validate, or every module with a connector fails
    /// its conformance test the moment it is built.
    #[test]
    fn a_published_manifest_with_connectors_validates() {
        let schema: Value = serde_json::from_str(MODULE_SCHEMA_V1).expect("schema");
        let validator = jsonschema::validator_for(&schema).expect("compiles");
        let manifest: Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/published-manifest-with-connectors.json"
        ))
        .expect("fixture");
        assert!(manifest["connectors"]["custom"].is_array());

        let errors: Vec<String> = validator
            .iter_errors(&manifest)
            .map(|e| e.to_string())
            .collect();
        assert!(errors.is_empty(), "{errors:?}");

        let mut wrong = manifest.clone();
        wrong["connectors"]["custom"][0]["id"] = json!("Not An Id");
        assert!(validator.iter_errors(&wrong).next().is_some());
    }
}
