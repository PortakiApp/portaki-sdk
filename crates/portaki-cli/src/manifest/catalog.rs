//! The catalogue metadata the code already knows how to state.
//!
//! `portaki.module.json` used to repeat the id, the version, the author, the name and the
//! description: all things that `portaki_module!(…)`, `Cargo.toml` and the i18n bundles already
//! carry. The build pulls them from there and fills in what the hand-written manifest does not
//! say; what it does say wins, for as long as it takes the modules to shed it.

use std::path::Path;

use anyhow::{Context, Result};
use serde_json::{json, Map, Value};

use portaki_sdk::permission;

use super::generator::EmissionFile;

/// Where `portaki build` drops the inferred catalogue.
pub const BUILT_CATALOG: &str = "target/portaki/catalog.json";

/// The catalogue inferred from the `module` emission and the i18n bundles.
///
/// The name and the description are the translations of their keys, keyed by short language
/// (`fr-FR` → `fr`); a language that lacks the key is left out rather than filled with the raw
/// key.
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
    for key in [
        "icon",
        "type",
        "maturity",
        "sortOrder",
        "audience",
        "hostScheduledSync",
    ] {
        if let Some(value) = extra.get(key) {
            catalog[key] = value.clone();
        }
    }
    // What the module feeds to another one: the text lives in i18n, under `feeds.<module>`.
    if let Some(feeds) = extra.get("feeds").and_then(Value::as_array) {
        catalog["feeds"] = feeds
            .iter()
            .map(|module| {
                let key = format!("feeds.{}", module.as_str().unwrap_or_default());
                json!({ "module": module, "what": translated(&Value::String(key)) })
            })
            .collect();
    }
    // The e-mail descriptions, translated; the rest of the entry comes from the built manifest.
    let emails: Vec<Value> = emissions
        .iter()
        .filter(|e| e.kind == "email")
        .filter_map(|e| {
            let key = e.data.get("descriptionKey")?;
            Some(json!({ "id": e.data["id"], "description": translated(key) }))
        })
        .collect();
    if !emails.is_empty() {
        catalog["emails"] = Value::Array(emails);
    }

    // `#[email_blocks]`: per email, the kinds of block the module adds.
    if let Some(declared) = emissions
        .iter()
        .find(|e| e.kind == "email_blocks")
        .and_then(|e| e.data["declared"].as_array())
    {
        catalog["emailBlocks"] = declared
            .iter()
            .filter_map(|entry| {
                Some((
                    entry["template"].as_str()?.to_string(),
                    entry["blocks"].clone(),
                ))
            })
            .collect::<Map<String, Value>>()
            .into();
    }

    // `#[email_vars]`: per template, the variables the module supplies.
    if let Some(declared) = emissions
        .iter()
        .find(|e| e.kind == "email_vars")
        .and_then(|e| e.data["declared"].as_array())
    {
        catalog["emailVars"] = declared
            .iter()
            .filter_map(|entry| {
                Some((
                    entry["template"].as_str()?.to_string(),
                    entry["vars"].clone(),
                ))
            })
            .collect::<Map<String, Value>>()
            .into();
    }

    if let Some(fields) = config_fields(emissions, &translated) {
        catalog["config"] = json!({ "fields": fields });
    }

    let (host, guest) = surfaces(
        emissions,
        data["id"].as_str().unwrap_or_default(),
        &translated,
    );
    if !host.is_empty() {
        catalog["hostSurfaces"] = Value::Array(host);
    }
    if !guest.is_empty() {
        catalog["guestSurfaces"] = Value::Array(guest);
    }
    catalog
}

/// The fields of `#[portaki_sdk::config]`, with their labels translated: the macro only carries
/// keys.
///
/// A `select` option is labelled by the key `<label>.<value>`. A list of rows gets its `item`
/// (translated sub-keys, identifier) from the `#[params]` shape of the row type.
fn config_fields(
    emissions: &[EmissionFile],
    translated: &dyn Fn(&Value) -> Value,
) -> Option<Vec<Value>> {
    let config = emissions.iter().find(|e| e.kind == "config")?;
    let mut fields = config.data["fields"].as_array()?.clone();
    portaki_sdk::config::resolve_items(&mut fields, |name| {
        emissions
            .iter()
            .find(|e| e.kind == "params" && e.data["name"] == name)
            .map(|e| e.data.clone())
    });
    Some(
        fields
            .iter()
            .map(|field| {
                let mut entry = field.clone();
                entry["label"] = translated(&field["label"]);
                if let Some(key) = field.get("description") {
                    entry["description"] = translated(key);
                }
                if let Some(options) = field["options"].as_array() {
                    entry["options"] = options
                        .iter()
                        .map(|value| {
                            let key = option_label_key(field, value);
                            json!({ "value": value, "label": translated(&Value::String(key)) })
                        })
                        .collect();
                }
                entry
            })
            .collect(),
    )
}

fn option_label_key(field: &Value, value: &Value) -> String {
    format!(
        "{}.{}",
        field["label"].as_str().unwrap_or_default(),
        value.as_str().unwrap_or_default()
    )
}

/// The navigation entries the `#[surface]`s describe, sorted so the manifest stays stable.
///
/// On the host side, one entry per `placement`; the `pathSegment` is the module id for the `main`
/// surface — the module's tab or card — and the surface id otherwise, unless an explicit `path`
/// overrides it. On the guest side, one entry per surface that has a `path`: the others render
/// without a link.
fn surfaces(
    emissions: &[EmissionFile],
    module_id: &str,
    translated: &dyn Fn(&Value) -> Value,
) -> (Vec<Value>, Vec<Value>) {
    let mut host = Vec::new();
    let mut guest = Vec::new();
    for surface in emissions.iter().filter(|e| e.kind == "surface") {
        let data = &surface.data;
        let Some(nav) = data["catalog"].as_object() else {
            continue;
        };
        let id = data["id"].as_str().unwrap_or_default();
        if data["context"] == "host" {
            let segment = nav
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or(if id == "main" { module_id } else { id });
            for placement in nav
                .get("placement")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let mut entry = json!({ "type": placement, "pathSegment": segment });
                if let Some(key) = nav.get("label_key") {
                    entry["label"] = translated(key);
                }
                if let Some(icon) = nav.get("icon") {
                    entry["icon"] = icon.clone();
                }
                if let Some(design) = nav.get("design_id") {
                    entry["hostUi"] = json!({ "designId": design });
                }
                host.push(entry);
            }
        } else if let Some(path) = nav.get("path") {
            let mut entry = json!({ "surfaceId": id, "path": path });
            for (from, to) in [
                ("label_key", "labelKey"),
                ("role", "role"),
                ("embeds", "embedsHostFragments"),
            ] {
                if let Some(value) = nav.get(from) {
                    entry[to] = value.clone();
                }
            }
            guest.push(entry);
        }
    }
    // The entries the dashboard draws without a surface.
    for nav in emissions.iter().filter(|e| e.kind == "nav") {
        let data = &nav.data;
        let mut entry = json!({ "type": data["placement"], "pathSegment": data["path"] });
        if let Some(key) = data.get("label_key") {
            entry["label"] = translated(key);
        }
        if let Some(icon) = data.get("icon") {
            entry["icon"] = icon.clone();
        }
        if let Some(design) = data.get("design_id") {
            entry["hostUi"] = json!({ "designId": design });
        }
        host.push(entry);
    }
    host.sort_by_key(|e| (e["pathSegment"].to_string(), e["type"].to_string()));
    // By route: the module's page before its sub-pages (`issue-report` before `issue-report/form`).
    guest.sort_by_key(|e| (e["path"].to_string(), e["surfaceId"].to_string()));
    (host, guest)
}

/// What the macros name without being able to check it: i18n keys, queries.
///
/// A macro sees neither the bundles nor the module's other attributes. The build, on the other
/// hand, has everything: a key missing from one language, or a misspelled query, stops here rather
/// than in a host's dashboard.
pub fn check_references(
    emissions: &[EmissionFile],
    i18n_dir: &Path,
    locales: &[String],
) -> anyhow::Result<()> {
    let mut keys: Vec<String> = Vec::new();
    for emission in emissions {
        let data = &emission.data;
        let named: Vec<Option<&Value>> = match emission.kind.as_str() {
            "module" => {
                for module in data["catalog"]["feeds"].as_array().into_iter().flatten() {
                    keys.push(format!("feeds.{}", module.as_str().unwrap_or_default()));
                }
                vec![data.get("displayName"), data.get("description")]
            }
            "surface" => vec![data["catalog"].get("label_key")],
            "nav" => vec![data.get("label_key")],
            "email" => vec![data.get("descriptionKey")],
            "config" => {
                for field in data["fields"].as_array().into_iter().flatten() {
                    for option in field["options"].as_array().into_iter().flatten() {
                        keys.push(option_label_key(field, option));
                    }
                }
                data["fields"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .flat_map(|field| [field.get("label"), field.get("description")])
                    .collect()
            }
            _ => Vec::new(),
        };
        keys.extend(
            named
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_string),
        );
    }

    let mut missing = Vec::new();
    for locale in locales {
        let bundle: Value = std::fs::read_to_string(i18n_dir.join(format!("{locale}.json")))
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        for key in &keys {
            if bundle.get(key).is_none() {
                missing.push(format!("{key} ({locale})"));
            }
        }
    }
    if !missing.is_empty() {
        anyhow::bail!(
            "i18n keys the module declares but does not translate: {}",
            missing.join(", ")
        );
    }

    let configs: Vec<&str> = emissions
        .iter()
        .filter(|e| e.kind == "config")
        .filter_map(|e| e.data["name"].as_str())
        .collect();
    if configs.len() > 1 {
        anyhow::bail!(
            "#[portaki_sdk::config] is on {} — a module has one configuration",
            configs.join(" and ")
        );
    }

    if emissions.iter().filter(|e| e.kind == "email_vars").count() > 1 {
        anyhow::bail!("#[email_vars] is on more than one function — a module has one emailContext");
    }

    if emissions
        .iter()
        .filter(|e| e.kind == "email_blocks")
        .count()
        > 1
    {
        anyhow::bail!(
            "#[email_blocks] is on more than one function — a module has one emailContext"
        );
    }

    // A module gives an email either variables or blocks. Both would need two `emailContext`
    // queries, and would put the same data twice in the same email — once in the body, once in a
    // tile. The platform ignores the blocks of a module that declares variables; say so here,
    // where the author can still choose.
    if emissions.iter().any(|e| e.kind == "email_vars")
        && emissions.iter().any(|e| e.kind == "email_blocks")
    {
        anyhow::bail!(
            "#[email_vars] and #[email_blocks] are both declared — a module gives an email \
             variables or blocks, not both (the platform reads only the variables)"
        );
    }

    let queries: Vec<&str> = emissions
        .iter()
        .filter(|e| e.kind == "query")
        .filter_map(|e| e.data["name"].as_str())
        .collect();
    for emission in emissions.iter().filter(|e| e.kind == "module") {
        let sync = &emission.data["catalog"]["hostScheduledSync"];
        for (attribute, field) in [
            ("scheduled_sync_sources", "sourcesQuery"),
            ("scheduled_sync_apply", "applyQuery"),
        ] {
            if let Some(query) = sync[field].as_str() {
                if !queries.contains(&query) {
                    anyhow::bail!("{attribute} names `{query}`, and no #[query] has that name");
                }
            }
        }
    }
    Ok(())
}

/// Every `portaki-sdk` feature and the permission it declares.
pub(crate) const FEATURE_PERMISSIONS: [(&str, &str); 7] = [
    ("kv", permission::KV),
    ("repo", permission::REPO),
    ("email", permission::EMAIL),
    ("events", permission::EVENTS),
    ("platform", permission::PLATFORM),
    ("guest-files", permission::GUEST_FILES),
    ("stay-guest-contact", permission::STAY_GUEST_CONTACT_READ),
];

/// The permissions the code claims: one per enabled `portaki-sdk` feature — the API it guards
/// does not exist without it — and `connectors:<id>` for each declared connector.
pub fn permissions(emissions: &[EmissionFile], sdk_features: &[String]) -> Vec<String> {
    let mut found: Vec<String> = FEATURE_PERMISSIONS
        .iter()
        .filter(|(feature, _)| sdk_features.iter().any(|f| f == feature))
        .map(|(_, permission)| permission.to_string())
        .chain(
            emissions
                .iter()
                .filter(|e| e.kind == "connector_builtin" || e.kind == "connector_custom")
                .filter_map(|e| e.data["id"].as_str())
                .map(|id| format!("{}{id}", permission::CONNECTORS_PREFIX)),
        )
        .collect();
    found.sort();
    found.dedup();
    found
}

/// Fills in the keys the hand-written manifest does not carry. What it does carry wins.
pub fn fill_catalog(raw: &str, catalog: &str) -> Result<String> {
    let catalog: Value = serde_json::from_str(catalog).context("parse built catalog")?;
    let Some(defaults) = catalog.as_object().filter(|c| !c.is_empty()) else {
        return Ok(raw.to_string());
    };
    let mut manifest: Value = serde_json::from_str(raw).context("parse module manifest")?;
    let Some(object) = manifest.as_object_mut() else {
        return Ok(raw.to_string());
    };
    let before = object.clone();
    for (key, value) in defaults {
        if is_empty(value) {
            continue;
        }
        match (object.get_mut(key), IDENTITY.iter().find(|(k, _)| k == key)) {
            (None, _) => {
                object.insert(key.clone(), value.clone());
            }
            // A permission the code claims is added; the hand-written ones stay.
            (Some(Value::Array(declared)), None) if key == "permissions" => {
                for permission in value.as_array().into_iter().flatten() {
                    if !declared.contains(permission) {
                        declared.push(permission.clone());
                    }
                }
            }
            // `config`: what the hand-written manifest says wins key by key — its whole
            // `fields` if it has one, the code's otherwise, next to its `globalAlert`.
            (Some(Value::Object(declared)), None) if key == "config" => {
                for (field, built) in value.as_object().into_iter().flatten() {
                    declared
                        .entry(field.clone())
                        .or_insert_with(|| built.clone());
                }
            }
            (Some(Value::Array(declared)), Some((_, fields))) => merge_entries(
                declared,
                value.as_array().map(Vec::as_slice).unwrap_or_default(),
                fields,
            ),
            _ => {}
        }
    }
    if *object == before {
        return Ok(raw.to_string());
    }
    serde_json::to_string_pretty(&manifest).context("serialise module manifest")
}

/// The lists that are merged entry by entry, and the fields that identify an entry.
const IDENTITY: [(&str, &[&str]); 3] = [
    ("hostSurfaces", &["type", "pathSegment"]),
    ("guestSurfaces", &["surfaceId"]),
    ("emails", &["id"]),
];

/// Adds the entries from the code that the manifest does not have, and fills in the fields it
/// leaves unsaid on the ones it does have. A hand-written entry the code knows nothing about —
/// `checklist`'s timeline task, which has no surface — is left exactly as it stands.
fn merge_entries(declared: &mut Vec<Value>, built: &[Value], identity: &[&str]) {
    let same = |a: &Value, b: &Value| identity.iter().all(|field| a.get(*field) == b.get(*field));
    for entry in built {
        match declared.iter_mut().find(|d| same(d, entry)) {
            Some(Value::Object(existing)) => {
                for (field, value) in entry.as_object().into_iter().flatten() {
                    if !is_empty(value) {
                        existing
                            .entry(field.clone())
                            .or_insert_with(|| value.clone());
                    }
                }
            }
            Some(_) => {}
            None => declared.push(entry.clone()),
        }
    }
}

/// A name without a single translation teaches the catalogue nothing.
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
    fn surfaces_become_navigation_entries() {
        let mut emissions = module();
        let surface = |data: serde_json::Value| EmissionFile {
            kind: "surface".into(),
            data,
        };
        emissions.push(surface(
            json!({ "context": "host", "id": "main", "renderFn": "a",
            "catalog": { "placement": ["property-workspace-tab"], "design_id": "report-v1",
                         "label_key": "nav.tab", "icon": "danger-triangle" } }),
        ));
        emissions.push(surface(
            json!({ "context": "host", "id": "issue-stats", "renderFn": "b",
            "catalog": { "placement": ["property-stats-detail", "property-stats-card"] } }),
        ));
        emissions.push(surface(
            json!({ "context": "guest", "id": "guest.form", "renderFn": "c",
            "catalog": { "path": "issue-report/form", "label_key": "nav.form" } }),
        ));
        emissions.push(surface(
            json!({ "context": "guest", "id": "home.card", "renderFn": "d" }),
        ));
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("fr-FR.json"), r#"{"nav.tab":"Onglet"}"#).expect("fr");

        let catalog = catalog_defaults(&emissions, dir.path(), &["fr-FR".to_string()]);

        assert_eq!(
            catalog["hostSurfaces"],
            json!([
                { "type": "property-workspace-tab", "pathSegment": "issue-report",
                  "label": { "fr": "Onglet" }, "icon": "danger-triangle",
                  "hostUi": { "designId": "report-v1" } },
                { "type": "property-stats-card", "pathSegment": "issue-stats" },
                { "type": "property-stats-detail", "pathSegment": "issue-stats" },
            ])
        );
        assert_eq!(
            catalog["guestSurfaces"],
            json!([{ "surfaceId": "guest.form", "path": "issue-report/form", "labelKey": "nav.form" }])
        );
    }

    #[test]
    fn surfaces_merge_entry_by_entry_and_the_hand_written_ones_stay() {
        let raw = r#"{"hostSurfaces":[
            {"type":"property-stats-card","pathSegment":"s","label":{"fr":"Main"}},
            {"type":"workspace-timeline-task","pathSegment":"tasks"}]}"#;
        let catalog = r#"{"hostSurfaces":[
            {"type":"property-stats-card","pathSegment":"s","label":{"fr":"Code"},"icon":"i"},
            {"type":"property-stats-detail","pathSegment":"s"}]}"#;

        let filled: serde_json::Value =
            serde_json::from_str(&fill_catalog(raw, catalog).expect("fill")).expect("parse");

        assert_eq!(
            filled["hostSurfaces"],
            json!([
                { "type": "property-stats-card", "pathSegment": "s", "label": { "fr": "Main" }, "icon": "i" },
                { "type": "workspace-timeline-task", "pathSegment": "tasks" },
                { "type": "property-stats-detail", "pathSegment": "s" },
            ])
        );
        assert_eq!(
            fill_catalog(&filled.to_string(), catalog).expect("again"),
            filled.to_string()
        );
    }

    #[test]
    fn permissions_come_from_sdk_features_and_connectors() {
        let mut emissions = module();
        emissions.push(EmissionFile {
            kind: "connector_custom".into(),
            data: json!({ "id": "nuki" }),
        });
        let features = ["repo", "email", "not-a-permission", "guest-files"].map(String::from);

        assert_eq!(
            super::permissions(&emissions, &features),
            ["connectors:nuki", "email", "guest:files", "repo"]
        );
    }

    #[test]
    fn permissions_add_to_the_hand_written_list() {
        let raw = r#"{"permissions":["platform","email"]}"#;
        let filled: serde_json::Value = serde_json::from_str(
            &fill_catalog(raw, r#"{"permissions":["email","repo"]}"#).expect("fill"),
        )
        .expect("parse");
        assert_eq!(filled["permissions"], json!(["platform", "email", "repo"]));
    }

    #[test]
    fn residual_fields_come_from_the_code_too() {
        let mut emissions = module();
        emissions[0].data["catalog"]["audience"] = json!("host");
        emissions[0].data["catalog"]["feeds"] = json!(["access-guide"]);
        emissions[0].data["catalog"]["hostScheduledSync"] = json!({ "platformFetch": true });
        emissions.push(EmissionFile {
            kind: "nav".into(),
            data: json!({ "placement": "workspace-timeline-task", "path": "tasks",
                          "label_key": "nav.tasks", "icon": "sparkles" }),
        });
        emissions.push(EmissionFile {
            kind: "email".into(),
            data: json!({ "id": "sync-failed", "descriptionKey": "email.syncFailed" }),
        });
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("fr-FR.json"),
            r#"{"nav.tasks":"Tâches","feeds.access-guide":"le code","email.syncFailed":"Échec"}"#,
        )
        .expect("fr");

        let catalog = catalog_defaults(&emissions, dir.path(), &["fr-FR".to_string()]);

        assert_eq!(catalog["audience"], "host");
        assert_eq!(catalog["hostScheduledSync"]["platformFetch"], true);
        assert_eq!(
            catalog["feeds"],
            json!([{ "module": "access-guide", "what": { "fr": "le code" } }])
        );
        assert_eq!(
            catalog["hostSurfaces"],
            json!([{ "type": "workspace-timeline-task", "pathSegment": "tasks",
                     "label": { "fr": "Tâches" }, "icon": "sparkles" }])
        );
        assert_eq!(
            catalog["emails"],
            json!([{ "id": "sync-failed", "description": { "fr": "Échec" } }])
        );
    }

    #[test]
    fn email_vars_become_a_map_by_template() {
        let mut emission = json!({ "kind": "email_vars", "declared": [
            { "template": "EmailTemplateKey::Arrival", "vars": ["EmailVar::WifiName", "EmailVar::HostPhone"] },
            { "template": "EmailTemplateKey::StayLink", "vars": ["EmailVar::WifiName"] },
        ]});
        super::super::generator::resolve_vocabulary(&mut emission).expect("resolve");
        let mut emissions = module();
        emissions.push(EmissionFile {
            kind: "email_vars".into(),
            data: emission,
        });
        let dir = tempfile::tempdir().expect("tempdir");

        let catalog = catalog_defaults(&emissions, dir.path(), &[]);

        assert_eq!(
            catalog["emailVars"],
            json!({ "arrival": ["wifiName", "hostPhone"], "stay-link": ["wifiName"] })
        );
        emissions.push(emissions.last().unwrap().clone());
        let twice = super::check_references(&emissions, dir.path(), &[]).unwrap_err();
        assert!(twice.to_string().contains("#[email_vars]"), "{twice}");
    }

    #[test]
    fn email_blocks_become_a_map_by_email() {
        let mut emission = json!({ "kind": "email_blocks", "declared": [
            { "template": "EmailTemplateKey::Arrival", "blocks": ["BlockType::Pairs", "BlockType::Info"] },
            { "template": "EmailTemplateKey::PostArrival", "blocks": ["BlockType::Checklist"] },
        ]});
        super::super::generator::resolve_vocabulary(&mut emission).expect("resolve");
        let mut emissions = module();
        emissions.push(EmissionFile {
            kind: "email_blocks".into(),
            data: emission,
        });
        let dir = tempfile::tempdir().expect("tempdir");

        let catalog = catalog_defaults(&emissions, dir.path(), &[]);

        assert_eq!(
            catalog["emailBlocks"],
            json!({ "arrival": ["pairs", "info"], "post-arrival": ["checklist"] })
        );
        emissions.push(emissions.last().unwrap().clone());
        let twice = super::check_references(&emissions, dir.path(), &[]).unwrap_err();
        assert!(twice.to_string().contains("#[email_blocks]"), "{twice}");
    }

    /// Un module donne à un e-mail des variables ou des blocs, pas les deux : il n'a qu'un
    /// `emailContext`, et la plateforme ne lirait que les variables.
    #[test]
    fn variables_and_blocks_together_are_refused() {
        let mut emissions = module();
        for kind in ["email_vars", "email_blocks"] {
            emissions.push(EmissionFile {
                kind: kind.into(),
                data: json!({ "kind": kind, "declared": [] }),
            });
        }
        let dir = tempfile::tempdir().expect("tempdir");

        let both = super::check_references(&emissions, dir.path(), &[]).unwrap_err();

        assert!(
            both.to_string()
                .contains("#[email_vars] and #[email_blocks]"),
            "{both}"
        );
    }

    #[test]
    fn a_declared_key_or_query_that_does_not_exist_stops_the_build() {
        let dir = tempfile::tempdir().expect("tempdir");
        let both = |fr: &str, en: &str| {
            std::fs::write(dir.path().join("fr-FR.json"), fr).expect("fr");
            std::fs::write(dir.path().join("en-US.json"), en).expect("en");
        };
        let locales = ["fr-FR".to_string(), "en-US".to_string()];
        let mut emissions = module();
        emissions[0].data["catalog"]["hostScheduledSync"] = json!({ "applyQuery": "applyFeeds" });
        emissions.push(EmissionFile {
            kind: "query".into(),
            data: json!({ "name": "applyFeeds" }),
        });
        let keys = r#"{"module.displayName":"x","module.description":"y"}"#;

        both(keys, keys);
        super::check_references(&emissions, dir.path(), &locales).expect("all there");

        both(keys, r#"{"module.displayName":"x"}"#);
        let missing = super::check_references(&emissions, dir.path(), &locales).unwrap_err();
        assert!(
            missing.to_string().contains("module.description (en-US)"),
            "{missing}"
        );

        both(keys, keys);
        emissions[0].data["catalog"]["hostScheduledSync"] = json!({ "applyQuery": "applyFeed" });
        let typo = super::check_references(&emissions, dir.path(), &locales).unwrap_err();
        assert!(typo.to_string().contains("applyFeed"), "{typo}");
    }

    fn with_config() -> Vec<EmissionFile> {
        let mut emissions = module();
        emissions.push(EmissionFile {
            kind: "config".into(),
            data: json!({ "name": "Config", "fields": [
                { "key": "ssid", "type": "text", "required": true, "recommended": false,
                  "label": "config.ssid" },
                { "key": "password", "type": "secret", "required": false, "recommended": true,
                  "label": "config.password", "description": "config.password.help" },
                { "key": "security", "type": "select", "required": false, "recommended": false,
                  "label": "config.security", "options": ["wpa2", "wep"] },
                { "key": "contacts", "type": "structured", "required": false,
                  "recommended": false, "label": "config.contacts", "itemType": "Contact" },
            ] }),
        });
        emissions
    }

    #[test]
    fn translated_row_fields_come_from_the_row_params() {
        let mut emissions = with_config();
        emissions.push(EmissionFile {
            kind: "params".into(),
            data: json!({ "kind": "params", "name": "Contact", "fields": [
                { "name": "id", "type": "string", "required": true },
                { "name": "role", "type": "ref", "ref": "I18nText", "required": true },
                { "name": "phone", "type": "string", "required": true },
            ] }),
        });
        let dir = tempfile::tempdir().expect("tempdir");
        let catalog = catalog_defaults(&emissions, dir.path(), &[]);
        assert_eq!(
            catalog["config"]["fields"][3],
            json!({ "key": "contacts", "type": "structured", "required": false,
                    "recommended": false, "label": {},
                    "item": { "id": "id", "localized": ["role"] } })
        );
    }

    const CONFIG_FR: &str = r#"{"config.ssid":"Nom du réseau","config.password":"Mot de passe",
        "config.password.help":"Au dos de la box","config.security":"Sécurité",
        "config.security.wpa2":"WPA2","config.security.wep":"WEP","config.contacts":"Contacts"}"#;

    #[test]
    fn config_labels_are_translated_from_the_bundles() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("fr-FR.json"), CONFIG_FR).expect("fr");

        let catalog = catalog_defaults(&with_config(), dir.path(), &["fr-FR".to_string()]);

        assert_eq!(
            catalog["config"],
            json!({ "fields": [
                { "key": "ssid", "type": "text", "required": true, "recommended": false,
                  "label": { "fr": "Nom du réseau" } },
                { "key": "password", "type": "secret", "required": false, "recommended": true,
                  "label": { "fr": "Mot de passe" }, "description": { "fr": "Au dos de la box" } },
                { "key": "security", "type": "select", "required": false, "recommended": false,
                  "label": { "fr": "Sécurité" }, "options": [
                    { "value": "wpa2", "label": { "fr": "WPA2" } },
                    { "value": "wep", "label": { "fr": "WEP" } } ] },
                { "key": "contacts", "type": "structured", "required": false,
                  "recommended": false, "label": { "fr": "Contacts" } },
            ] })
        );
    }

    #[test]
    fn a_config_label_missing_from_a_bundle_stops_the_build() {
        let dir = tempfile::tempdir().expect("tempdir");
        let module_keys = r#""module.displayName":"x","module.description":"y""#;
        let fr = CONFIG_FR.replacen('{', &format!("{{{module_keys},"), 1);
        std::fs::write(dir.path().join("fr-FR.json"), &fr).expect("fr");
        std::fs::write(
            dir.path().join("en-US.json"),
            fr.replace(r#""config.security.wep":"WEP","#, ""),
        )
        .expect("en");
        let locales = ["fr-FR".to_string(), "en-US".to_string()];

        let missing = super::check_references(&with_config(), dir.path(), &locales).unwrap_err();
        assert_eq!(
            missing.to_string(),
            "i18n keys the module declares but does not translate: config.security.wep (en-US)"
        );

        let mut two = with_config();
        two.push(EmissionFile {
            kind: "config".into(),
            data: json!({ "name": "Other", "fields": [] }),
        });
        std::fs::write(dir.path().join("en-US.json"), &fr).expect("en");
        let error = super::check_references(&two, dir.path(), &locales).unwrap_err();
        assert!(error.to_string().contains("Config and Other"), "{error}");
    }

    #[test]
    fn hand_written_config_fields_win_and_the_rest_is_filled() {
        let built = r#"{"config":{"fields":[{"key":"ssid"}]}}"#;
        let alert_only = r#"{"config":{"globalAlert":{"type":"info"}}}"#;
        let filled: serde_json::Value =
            serde_json::from_str(&fill_catalog(alert_only, built).expect("fill")).expect("parse");
        assert_eq!(
            filled["config"],
            json!({ "globalAlert": { "type": "info" }, "fields": [{ "key": "ssid" }] })
        );

        let hand_written = r#"{"config":{"fields":[{"key":"legacy"}]}}"#;
        assert_eq!(
            fill_catalog(hand_written, built).expect("fill"),
            hand_written
        );
    }

    #[test]
    fn nothing_to_fill_leaves_the_manifest_untouched() {
        let raw = r#"{"id":"issue-report"}"#;
        assert_eq!(fill_catalog(raw, r#"{"id":"x"}"#).expect("fill"), raw);
        assert_eq!(fill_catalog(raw, "{}").expect("fill"), raw);
    }
}
