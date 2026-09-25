//! The public catalogue previews: every guest surface the booklet serves, rendered on sample data.
//!
//! A module's page on portaki.app shows what the guest will see — not a screenshot: the SDUI tree
//! the module renders, painted by the booklet engine. It is rendered here, by the code of the
//! Wasm binary, on a sample configuration — never on a host's data.
//!
//! `previews.json` is committed next to the manifest: a preview is reviewed in a PR like the rest,
//! `portaki build` ships it in the artifact, and the registry serves it with the version. The test
//! fails when the file no longer matches what the module renders; to write it again:
//!
//! ```sh
//! PORTAKI_UPDATE_PREVIEWS=1 cargo test --test previews
//! ```
//!
//! One test, `tests/previews.rs`:
//!
//! ```ignore
//! use portaki_test_utils::previews;
//!
//! portaki_test_utils::link_module!(); // the module's surfaces, linked into this test
//!
//! #[test]
//! fn previews_match_the_rendered_surfaces() {
//!     let root = env!("CARGO_MANIFEST_DIR");
//!     previews::check_all(root, previews::guest(root).with_config(&sample_config()));
//! }
//! ```
//!
//! [`check_all`] renders every guest surface declared with a `path` through the SDK's own
//! dispatch; [`check`] takes surfaces rendered by hand, when one needs its own context.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use chrono::{DateTime, Utc};
use portaki_sdk::context::StayContext;
use portaki_sdk::sdui::surface::Surface;
use portaki_sdk::wasm::registry::{self, HandlerKind};
use serde_json::{json, Map, Value};
use uuid::Uuid;

use crate::{MockContext, MockContextBuilder, Property};

/// The language previews are rendered in.
pub const LOCALE: &str = "fr-FR";

/// The previews' instant: the day before check-in.
pub const NOW: &str = "2026-05-31T10:00:00Z";

/// The file, at the module root.
pub const FILE: &str = "previews.json";

/// An RFC 3339 instant, in UTC.
pub fn at(rfc3339: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(rfc3339)
        .expect("RFC 3339 instant")
        .with_timezone(&Utc)
}

/// A guest context in French, the module's translations loaded.
///
/// The fixture property, a stay from 1 to 8 June 2026 and a clock frozen the day before check-in:
/// a preview depends neither on the day the test runs nor on a random id.
pub fn guest(module_root: &str) -> MockContextBuilder {
    let stay = StayContext {
        stay_id: Uuid::from_u128(0x2222_2222_2222_2222_2222_2222_2222_2222),
        checkin_at: Some(at("2026-06-01T15:00:00Z")),
        checkout_at: Some(at("2026-06-08T10:00:00Z")),
        ..StayContext::default()
    };
    fr_bundle(module_root)
        .into_iter()
        .fold(MockContext::guest(), |builder, (key, value)| {
            builder.with_translation(key, value.as_str().unwrap_or_default())
        })
        .with_property(Property::default())
        .with_stay(stay)
        .with_now(at(NOW))
}

/// Renders every guest route — a `#[surface(guest, path = …)]` — with `context`, and compares
/// `previews.json` to it (or writes it, under `PORTAKI_UPDATE_PREVIEWS`). A surface that fails —
/// an `Err`, which the SDK would show as its error state — fails the test.
pub fn check_all(module_root: &str, context: MockContextBuilder) {
    let rendered = routes()
        .into_keys()
        .map(|surface_id| {
            let declaration = registry::declarations()
                .find(|d| d.kind == HandlerKind::Surface && d.name == surface_id)
                .expect("a declared route");
            let tree = context.clone().run_with(|ctx, host| {
                let surface = (declaration.dispatch)(ctx, json!({}))
                    .unwrap_or_else(|error| panic!("{surface_id}: {error}"));
                let failures: Vec<_> = host
                    .logs()
                    .into_iter()
                    .filter(|line| line.level == "error")
                    .collect();
                assert!(failures.is_empty(), "{surface_id} failed: {failures:?}");
                surface["root"].clone()
            });
            (surface_id, tree)
        })
        .collect();
    compare(module_root, rendered);
}

/// [`check_all`] with surfaces rendered by the test: one per guest route, by surface id.
pub fn check(module_root: &str, rendered: Vec<(&str, Surface)>) {
    let rendered = rendered
        .into_iter()
        .map(|(id, surface)| {
            let tree = serde_json::to_value(&surface.root).expect("SDUI tree");
            let id = routes()
                .into_keys()
                .find(|route| *route == id)
                .unwrap_or_else(|| panic!("`{id}` is not a guest surface with a path"));
            (id, tree)
        })
        .collect();
    compare(module_root, rendered);
}

/// The guest surfaces the booklet serves, by id: their `#[surface]` gives a `path`.
fn routes() -> BTreeMap<&'static str, Value> {
    let routes: BTreeMap<_, _> = registry::declarations()
        .filter(|d| d.kind == HandlerKind::Surface && d.context == "guest")
        .filter_map(|d| {
            let catalog: Value = serde_json::from_str(d.catalog).ok()?;
            catalog["path"].is_string().then_some((d.name, catalog))
        })
        .collect();
    assert!(
        !routes.is_empty(),
        "no guest surface with a path is linked into this test binary — name the module crate \
         (`use my_module as _;`) or declare one"
    );
    routes
}

fn compare(module_root: &str, mut rendered: Vec<(&'static str, Value)>) {
    rendered.sort_by_key(|(id, _)| *id);
    let declared = routes();
    let got: Vec<&str> = rendered.iter().map(|(id, _)| *id).collect();
    let ids: Vec<&str> = declared.keys().copied().collect();
    assert_eq!(got, ids, "one preview per guest surface the booklet serves");

    let bundle = fr_bundle(module_root);
    let mut seen = Vec::new();
    let surfaces: Vec<Value> = rendered
        .into_iter()
        .map(|(surface_id, mut tree)| {
            stable_uuids(&mut tree, &mut seen);
            let label_key = declared[surface_id]["label_key"]
                .as_str()
                .unwrap_or_default();
            let mut keys = i18n_refs(&tree);
            keys.insert(label_key.to_string());
            let i18n: Map<String, Value> = keys
                .into_iter()
                .filter_map(|key| bundle.get(&key).map(|value| (key, value.clone())))
                .collect();
            json!({
                "surfaceId": surface_id,
                "title": bundle.get(label_key).cloned().unwrap_or(Value::Null),
                "tree": tree,
                "i18n": i18n,
            })
        })
        .collect();

    let expected = serde_json::to_string_pretty(&json!({ "locale": LOCALE, "surfaces": surfaces }))
        .expect("json")
        + "\n";
    let path = Path::new(module_root).join(FILE);
    if std::env::var_os("PORTAKI_UPDATE_PREVIEWS").is_some() {
        fs::write(&path, &expected).expect("write previews.json");
        return;
    }
    let current = fs::read_to_string(&path).unwrap_or_default();
    assert!(
        current == expected,
        "{FILE} no longer matches the rendering — PORTAKI_UPDATE_PREVIEWS=1 cargo test --test previews"
    );
}

fn fr_bundle(module_root: &str) -> Map<String, Value> {
    let path = Path::new(module_root)
        .join("i18n")
        .join(format!("{LOCALE}.json"));
    let raw =
        fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    serde_json::from_str(&raw).expect("fr-FR bundle")
}

/// Replaces every UUID of the tree by a stable one, numbered in order of appearance — a row
/// stored under `Uuid::new_v4` shows in its actions, and would change the preview on every run.
fn stable_uuids(value: &mut Value, seen: &mut Vec<Uuid>) {
    match value {
        Value::String(text) => {
            let mut out = String::with_capacity(text.len());
            let mut rest = text.as_str();
            while !rest.is_empty() {
                match rest.get(..36).and_then(|head| Uuid::try_parse(head).ok()) {
                    Some(id) => {
                        let index = seen.iter().position(|s| *s == id).unwrap_or_else(|| {
                            seen.push(id);
                            seen.len() - 1
                        });
                        out.push_str(&Uuid::from_u128(index as u128 + 1).to_string());
                        rest = &rest[36..];
                    }
                    None => {
                        let next = rest.chars().next().expect("not empty");
                        out.push(next);
                        rest = &rest[next.len_utf8()..];
                    }
                }
            }
            *text = out;
        }
        Value::Array(items) => items.iter_mut().for_each(|item| stable_uuids(item, seen)),
        Value::Object(fields) => fields
            .values_mut()
            .for_each(|item| stable_uuids(item, seen)),
        _ => {}
    }
}

/// Every `i18n:` key the tree references.
fn i18n_refs(tree: &Value) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    let mut stack = vec![tree];
    while let Some(value) = stack.pop() {
        match value {
            Value::String(text) => {
                if let Some(key) = text.strip_prefix("i18n:") {
                    keys.insert(key.to_string());
                }
            }
            Value::Array(items) => stack.extend(items),
            Value::Object(fields) => stack.extend(fields.values()),
            _ => {}
        }
    }
    keys
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uuids_are_renumbered_in_order() {
        let a = "7c1e8a52-9d1c-4a3e-8f5a-2b6d9e0c1f23";
        let b = "0d9f3b1a-2c4e-4f6a-8b1c-3d5e7f9a1b2c";
        let mut tree = json!({ "x": format!("row:{a}"), "y": [b, a] });
        stable_uuids(&mut tree, &mut Vec::new());
        assert_eq!(
            tree,
            json!({
                "x": "row:00000000-0000-0000-0000-000000000001",
                "y": ["00000000-0000-0000-0000-000000000002", "00000000-0000-0000-0000-000000000001"],
            })
        );
    }

    #[test]
    fn i18n_keys_are_collected_anywhere_in_the_tree() {
        let tree = json!({ "a": "i18n:x", "b": [{ "c": "i18n:y" }, "plain"] });
        let keys: Vec<String> = i18n_refs(&tree).into_iter().collect();
        assert_eq!(keys, ["x", "y"]);
    }
}
