//! Les métadonnées de catalogue que le code sait déjà dire.
//!
//! `portaki.module.json` répétait l'id, la version, l'auteur, le nom et la description : autant
//! de choses que `portaki_module!(…)`, `Cargo.toml` et les bundles i18n portent déjà. Le build
//! les en tire et comble ce que le manifeste écrit à la main ne dit pas ; ce qu'il dit l'emporte,
//! le temps que les modules s'en délestent.

use std::path::Path;

use anyhow::{Context, Result};
use serde_json::{json, Map, Value};

use portaki_sdk::permission;

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
    // Ce que le module fournit à un autre : le texte vit dans i18n, sous `feeds.<module>`.
    if let Some(feeds) = extra.get("feeds").and_then(Value::as_array) {
        catalog["feeds"] = feeds
            .iter()
            .map(|module| {
                let key = format!("feeds.{}", module.as_str().unwrap_or_default());
                json!({ "module": module, "what": translated(&Value::String(key)) })
            })
            .collect();
    }
    // Les descriptions d'e-mails, traduites ; le reste de l'entrée vient du manifeste du build.
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

/// Les entrées de navigation que les `#[surface]` décrivent, triées pour un manifeste stable.
///
/// Côté hôte, une entrée par `placement` ; le `pathSegment` est l'id du module pour la surface
/// `main` — l'onglet ou la fiche du module —, l'id de la surface sinon, sauf `path` explicite.
/// Côté invité, une entrée par surface qui a une `path` : les autres se rendent sans lien.
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
    // Les entrées que le tableau de bord dessine sans surface.
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
    // Par route : la page du module avant ses sous-pages (`issue-report` avant `issue-report/form`).
    guest.sort_by_key(|e| (e["path"].to_string(), e["surfaceId"].to_string()));
    (host, guest)
}

/// Ce que les macros nomment sans pouvoir le vérifier : des clés i18n, des queries.
///
/// Une macro ne voit ni les bundles ni les autres attributs du module. Le build, lui, a tout : une
/// clé absente d'une langue ou une query mal orthographiée s'arrête ici, pas dans le dashboard
/// d'un hôte.
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

/// Chaque feature de `portaki-sdk` et la permission qu'elle déclare.
const FEATURE_PERMISSIONS: [(&str, &str); 7] = [
    ("kv", permission::KV),
    ("repo", permission::REPO),
    ("email", permission::EMAIL),
    ("events", permission::EVENTS),
    ("platform", permission::PLATFORM),
    ("guest-files", permission::GUEST_FILES),
    ("stay-guest-contact", permission::STAY_GUEST_CONTACT_READ),
];

/// Les permissions que le code réclame : une par feature de `portaki-sdk` activée — l'API
/// qu'elle garde n'existe pas sans elle —, et `connectors:<id>` pour chaque connecteur déclaré.
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
    let before = object.clone();
    for (key, value) in defaults {
        if is_empty(value) {
            continue;
        }
        match (object.get_mut(key), IDENTITY.iter().find(|(k, _)| k == key)) {
            (None, _) => {
                object.insert(key.clone(), value.clone());
            }
            // Une permission que le code réclame s'ajoute ; celles écrites à la main restent.
            (Some(Value::Array(declared)), None) if key == "permissions" => {
                for permission in value.as_array().into_iter().flatten() {
                    if !declared.contains(permission) {
                        declared.push(permission.clone());
                    }
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

/// Les listes fusionnées entrée par entrée, et les champs qui identifient une entrée.
const IDENTITY: [(&str, &[&str]); 3] = [
    ("hostSurfaces", &["type", "pathSegment"]),
    ("guestSurfaces", &["surfaceId"]),
    ("emails", &["id"]),
];

/// Ajoute les entrées du code que le manifeste n'a pas, et comble les champs qu'il tait sur
/// celles qu'il a. Une entrée écrite à la main que le code ignore — la tâche de frise de
/// `checklist`, qui n'a pas de surface — reste telle quelle.
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

    #[test]
    fn nothing_to_fill_leaves_the_manifest_untouched() {
        let raw = r#"{"id":"issue-report"}"#;
        assert_eq!(fill_catalog(raw, r#"{"id":"x"}"#).expect("fill"), raw);
        assert_eq!(fill_catalog(raw, "{}").expect("fill"), raw);
    }
}
