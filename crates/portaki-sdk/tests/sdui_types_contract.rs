//! Derives `contracts/sdui_types.json` from the Rust, and freezes it.
//!
//! `sdui_primitives.json` names the types of its fields — `"action": "Action"` — without ever
//! describing them. A consumer of the contract can therefore render a primitive but cannot edit an
//! `Action` other than as raw JSON: it does not know its variants. This document fills that gap.
//!
//! It is **derived, not written**: the definitions live in `sdui/common.rs` and `sdui/action.rs`,
//! and that is where they are read from. A copy kept by hand would have drifted — the dashboard
//! has already paid for that on its six common fields.
//!
//! Regenerate after touching a type: `BLESS=1 cargo test -p portaki-sdk --test
//! sdui_types_contract`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

use serde_json::{json, Map, Value};
use syn::{Attribute, Expr, Fields, Item, Lit, Type};

/// The files where the types the contract names live.
const SOURCES: [&str; 2] = ["src/sdui/common.rs", "src/sdui/action.rs"];

/// What `build.rs` puts on every primitive without the contract saying so.
///
/// Their type is referenced by every primitive and by no declared field: without this root, the
/// transitive closure would miss them and a consumer would have nothing to edit `tone` with.
const COMMON_FIELD_TYPES: [&str; 5] = [
    "Tone",
    "Emphasis",
    "SurfaceLevel",
    "Animation",
    "Visibility",
];

/// The types the contract treats as scalars: they have nothing to describe.
fn is_scalar(ty: &str) -> bool {
    matches!(
        ty,
        "String" | "bool" | "f64" | "u32" | "i64" | "u64" | "usize" | "Value" | "Component"
    )
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

// ── Reading the serde attributes ─────────────────────────────────────────────

/// `#[serde(rename = "…")]` carried by a variant or a field.
fn serde_rename(attrs: &[Attribute]) -> Option<String> {
    serde_string_attr(attrs, "rename")
}

/// `#[serde(rename_all = "…")]` carried by the container.
fn serde_rename_all(attrs: &[Attribute]) -> Option<String> {
    serde_string_attr(attrs, "rename_all")
}

/// `#[serde(tag = "…")]` — an internally tagged enum is flat on the wire.
fn serde_tag(attrs: &[Attribute]) -> Option<String> {
    serde_string_attr(attrs, "tag")
}

fn serde_string_attr(attrs: &[Attribute], key: &str) -> Option<String> {
    let mut found = None;
    for attr in attrs.iter().filter(|a| a.path().is_ident("serde")) {
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident(key) {
                if let Ok(value) = meta.value() {
                    if let Ok(Expr::Lit(lit)) = value.parse::<Expr>() {
                        if let Lit::Str(text) = lit.lit {
                            found = Some(text.value());
                        }
                    }
                }
            } else {
                // `alias`, `skip_serializing_if`, `default`…: consume the value if there is
                // one, so that the parse does not stop at the first neighbour.
                let _ = meta.value().and_then(|v| v.parse::<Expr>());
            }
            Ok(())
        });
        if found.is_some() {
            break;
        }
    }
    found
}

/// True when `#[portaki_sdk_macros::wire]` covers the item.
///
/// The macro adds `rename_all = "camelCase"` **when the container does not already carry one**.
/// Its effect is therefore invisible in the source, and ignoring it would give `snake_case` field
/// names that nobody emits.
fn has_wire(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        let path = attr.path();
        path.segments
            .last()
            .map(|s| s.ident == "wire")
            .unwrap_or(false)
    })
}

/// The container's effective convention, the macro included.
///
/// On an enum it names the **variants** — not the fields those variants carry. The distinction is
/// serde's own, and it is plain to see on `Action`: the variant is written `openOverlay`, its
/// field stays `surface_render`.
fn effective_rename_all(attrs: &[Attribute]) -> Option<String> {
    serde_rename_all(attrs).or_else(|| has_wire(attrs).then(|| "camelCase".to_string()))
}

/// The convention for the fields **carried by a variant**.
///
/// Renaming the variants does not rename their fields: that takes `rename_all_fields` on the enum,
/// or `rename_all` on the variant. Without one of the two, serde writes the Rust identifier as it
/// stands — and `wire` changes nothing there, since all it puts on is `rename_all`.
fn variant_field_rename_all(container: &[Attribute], variant: &[Attribute]) -> Option<String> {
    serde_string_attr(container, "rename_all_fields").or_else(|| serde_rename_all(variant))
}

fn apply_case(name: &str, convention: Option<&str>) -> String {
    match convention {
        Some("snake_case") => to_snake(name),
        Some("camelCase") => to_camel(name),
        Some("kebab-case") => to_snake(name).replace('_', "-"),
        Some("SCREAMING_SNAKE_CASE") => to_snake(name).to_uppercase(),
        Some("lowercase") => name.to_lowercase(),
        // With no convention, serde writes the identifier as it stands.
        _ => name.to_string(),
    }
}

fn to_snake(name: &str) -> String {
    let mut out = String::new();
    for (index, ch) in name.char_indices() {
        if ch.is_uppercase() {
            if index != 0 {
                out.push('_');
            }
            out.extend(ch.to_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

fn to_camel(name: &str) -> String {
    let snake = to_snake(name);
    let mut parts = snake.split('_');
    let mut out = parts.next().unwrap_or_default().to_string();
    for part in parts {
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            out.extend(first.to_uppercase());
            out.push_str(chars.as_str());
        }
    }
    out
}

// ── Reading the types ────────────────────────────────────────────────────────

/// The name of the type a field carries, with `Option` and `Vec` stripped off.
///
/// Also returns whether the field is optional: that is what tells a field an editor may leave out
/// apart from a field it has to ask for.
fn field_type(ty: &Type) -> (String, bool, bool) {
    let rendered = quote_type(ty);
    let (inner, optional) = match strip_wrapper(&rendered, "Option") {
        Some(inner) => (inner, true),
        None => (rendered, false),
    };
    let (inner, repeated) = match strip_wrapper(&inner, "Vec") {
        Some(item) => (item, true),
        None => (inner, false),
    };
    // `Option<Box<Component>>`: the `Box` sits under the wrapper, not in front of it.
    let inner = strip_wrapper(&inner, "Box").unwrap_or(inner);
    (inner, optional, repeated)
}

fn strip_wrapper(rendered: &str, wrapper: &str) -> Option<String> {
    rendered
        .strip_prefix(wrapper)?
        .strip_prefix('<')?
        .strip_suffix('>')
        .map(str::to_string)
}

/// The type rendered without spaces, without `Box`, and reduced to its last path segment.
///
/// `Box` exists only to give a finite size to a primitive that holds another one (`build.rs` adds
/// it): it says nothing about the wire, and the contract does not name it.
fn quote_type(ty: &Type) -> String {
    use quote::ToTokens;
    let compact: String = ty
        .to_token_stream()
        .to_string()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    let unboxed = strip_wrapper(&compact, "Box").unwrap_or(compact);
    last_segment(&unboxed)
}

/// `crate::sdui::common::Tone` → `Tone`, leaving the generic parameters intact.
fn last_segment(rendered: &str) -> String {
    match rendered.split_once('<') {
        Some((head, tail)) => {
            let head = head.rsplit("::").next().unwrap_or(head);
            format!("{head}<{}", last_segment_tail(tail))
        }
        None => rendered.rsplit("::").next().unwrap_or(rendered).to_string(),
    }
}

fn last_segment_tail(tail: &str) -> String {
    match tail.strip_suffix('>') {
        Some(inner) => format!("{}>", last_segment(inner)),
        None => tail.to_string(),
    }
}

#[derive(Default)]
struct Catalog {
    types: BTreeMap<String, Value>,
    /// The types a field names but that nothing defines — expected to be scalars, otherwise a gap.
    referenced: BTreeSet<String>,
}

fn read_sources() -> Catalog {
    let mut catalog = Catalog::default();
    for relative in SOURCES {
        let path = manifest_dir().join(relative);
        let text = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("lecture de {}: {error}", path.display()));
        let file = syn::parse_file(&text)
            .unwrap_or_else(|error| panic!("analyse de {}: {error}", path.display()));
        for item in file.items {
            match item {
                Item::Enum(item) => {
                    let convention = effective_rename_all(&item.attrs);
                    let mut variants = Vec::new();
                    for variant in &item.variants {
                        let wire = serde_rename(&variant.attrs).unwrap_or_else(|| {
                            apply_case(&variant.ident.to_string(), convention.as_deref())
                        });
                        let mut entry = Map::new();
                        entry.insert("name".into(), json!(wire));
                        if let Fields::Named(named) = &variant.fields {
                            let inner = variant_field_rename_all(&item.attrs, &variant.attrs);
                            let fields =
                                read_named_fields(named, inner.as_deref(), &mut catalog.referenced);
                            entry.insert("fields".into(), Value::Object(fields));
                        }
                        variants.push(Value::Object(entry));
                    }
                    let mut definition = Map::new();
                    definition.insert("kind".into(), json!("enum"));
                    if let Some(tag) = serde_tag(&item.attrs) {
                        definition.insert("tag".into(), json!(tag));
                    }
                    definition.insert("variants".into(), Value::Array(variants));
                    catalog
                        .types
                        .insert(item.ident.to_string(), Value::Object(definition));
                }
                Item::Struct(item) => {
                    // `pub struct VisibilityExpr(pub String);` — serde serialises it
                    // transparently, so on the wire it is a string. Saying so keeps a consumer
                    // from hunting for a structure that never shows up.
                    if let Fields::Unnamed(unnamed) = &item.fields {
                        if unnamed.unnamed.len() == 1 {
                            let (inner, _, repeated) = field_type(&unnamed.unnamed[0].ty);
                            catalog.referenced.insert(inner.clone());
                            let mut definition = Map::new();
                            definition.insert("kind".into(), json!("newtype"));
                            definition.insert("type".into(), json!(inner));
                            if repeated {
                                definition.insert("repeated".into(), json!(true));
                            }
                            catalog
                                .types
                                .insert(item.ident.to_string(), Value::Object(definition));
                        }
                    }
                    if let Fields::Named(named) = &item.fields {
                        let convention = effective_rename_all(&item.attrs);
                        let fields = read_named_fields(
                            named,
                            convention.as_deref(),
                            &mut catalog.referenced,
                        );
                        let mut definition = Map::new();
                        definition.insert("kind".into(), json!("struct"));
                        definition.insert("fields".into(), Value::Object(fields));
                        catalog
                            .types
                            .insert(item.ident.to_string(), Value::Object(definition));
                    }
                }
                _ => {}
            }
        }
    }
    // The icons: a closed list held in `vocab.rs` by a macro that syn does not expand. The
    // contract therefore reads it off the type itself — this is what every shell must know how to
    // draw.
    catalog.types.insert(
        "IconName".into(),
        vocabulary::<portaki_sdk::vocab::IconName>(),
    );
    catalog
}

/// An enum from `vocab`, described the way serde writes it: one string per variant.
fn vocabulary<V: portaki_sdk::vocab::Vocabulary>() -> Value {
    let variants: Vec<Value> = V::ALL.iter().map(|v| json!({ "name": v.wire() })).collect();
    json!({ "kind": "enum", "variants": variants })
}

fn read_named_fields(
    named: &syn::FieldsNamed,
    convention: Option<&str>,
    referenced: &mut BTreeSet<String>,
) -> Map<String, Value> {
    let mut fields = Map::new();
    for field in &named.named {
        let Some(ident) = field.ident.as_ref() else {
            continue;
        };
        let wire = serde_rename(&field.attrs)
            .unwrap_or_else(|| apply_case(&ident.to_string(), convention));
        let (type_name, optional, repeated) = field_type(&field.ty);
        referenced.insert(type_name.clone());
        let mut entry = Map::new();
        entry.insert("type".into(), json!(type_name));
        if optional {
            entry.insert("optional".into(), json!(true));
        }
        if repeated {
            entry.insert("repeated".into(), json!(true));
        }
        fields.insert(wire, Value::Object(entry));
    }
    fields
}

// ── Transitive closure from the primitives ───────────────────────────────────

/// The types a consumer of the contract may run into, and only those.
///
/// Starts from the fields `sdui_primitives.json` declares and from the common fields, then follows
/// the references. `Action` on its own pulls in `OverlayPresentation` and `OverlayArgs`: publishing
/// the root without its descendants would leave the editor half-way.
fn reachable_types(catalog: &Catalog) -> BTreeSet<String> {
    let primitives_path = manifest_dir().join("sdui_primitives.json");
    let raw = fs::read_to_string(&primitives_path).expect("lecture de sdui_primitives.json");
    let primitives: Vec<Value> =
        serde_json::from_str(&raw).expect("analyse de sdui_primitives.json");

    let mut queue: Vec<String> = COMMON_FIELD_TYPES.iter().map(|s| s.to_string()).collect();
    for primitive in &primitives {
        if let Some(fields) = primitive.get("fields").and_then(Value::as_object) {
            for declared in fields.values().filter_map(Value::as_str) {
                let name = declared
                    .strip_prefix("Vec<")
                    .and_then(|s| s.strip_suffix('>'))
                    .unwrap_or(declared);
                queue.push(name.to_string());
            }
        }
    }

    let mut reached = BTreeSet::new();
    while let Some(name) = queue.pop() {
        if is_scalar(&name) || !reached.insert(name.clone()) {
            continue;
        }
        let Some(definition) = catalog.types.get(&name) else {
            continue;
        };
        let mut follow = |entry: &Value| {
            if let Some(fields) = entry.get("fields").and_then(Value::as_object) {
                for field in fields.values() {
                    if let Some(next) = field.get("type").and_then(Value::as_str) {
                        queue.push(next.to_string());
                    }
                }
            }
            // A newtype carries its type at the same level as its `kind`.
            if let Some(next) = entry.get("type").and_then(Value::as_str) {
                queue.push(next.to_string());
            }
        };
        follow(definition);
        if let Some(variants) = definition.get("variants").and_then(Value::as_array) {
            for variant in variants {
                follow(variant);
            }
        }
    }
    reached.retain(|name| !is_scalar(name));
    reached
}

fn generate() -> Value {
    let catalog = read_sources();
    let reachable = reachable_types(&catalog);
    let mut out = Map::new();
    for name in &reachable {
        let definition = catalog.types.get(name).unwrap_or_else(|| {
            panic!(
                "le contrat nomme `{name}`, qu'aucun des fichiers lus ne définit — \
                 ajoutez-le à SOURCES ou à la liste des scalaires"
            )
        });
        out.insert(name.clone(), definition.clone());
    }
    Value::Object(out)
}

fn committed_path() -> PathBuf {
    manifest_dir().join("../../contracts/sdui_types.json")
}

#[test]
fn le_document_des_types_est_a_jour() {
    let generated = generate();
    let rendered = format!("{}\n", serde_json::to_string_pretty(&generated).unwrap());

    if std::env::var_os("BLESS").is_some() {
        fs::write(committed_path(), &rendered).expect("écriture de contracts/sdui_types.json");
        return;
    }

    let committed = fs::read_to_string(committed_path()).unwrap_or_default();
    assert_eq!(
        committed, rendered,
        "\n`contracts/sdui_types.json` ne correspond plus aux types Rust.\n\
         Regénérez-le : BLESS=1 cargo test -p portaki-sdk --test sdui_types_contract\n"
    );
}

/// What the generator claims about the wire must be what serde actually does.
///
/// The naming convention is **replayed** here from the attributes, `wire` included; without this
/// test it would be an article of faith. So real values are serialised, and compared.
#[test]
fn les_noms_sur_le_fil_sont_ceux_que_serde_emet() {
    use portaki_sdk::sdui::action::Action;
    use portaki_sdk::sdui::common::{MapInteractionMode, TemperatureUnit, Tone};

    let types = generate();

    let action = serde_json::to_value(Action::External {
        url: "https://portaki.app".into(),
    })
    .unwrap();
    assert_eq!(action["type"], json!("external"));
    let variants = types["Action"]["variants"].as_array().unwrap();
    assert!(
        variants.iter().any(|v| v["name"] == json!("external")),
        "le générateur ignore la variante que serde écrit `external`"
    );
    assert_eq!(types["Action"]["tag"], json!("type"));

    // The trap this test exists to catch: `rename_all` on an enum names its variants and **not**
    // the fields they carry. `command` is in camelCase, `module_id` stays in snake_case. A contract
    // announcing `moduleId` would have consumers write a field the Rust never reads back.
    let command = serde_json::to_value(Action::Command {
        module_id: "weather".into(),
        name: "refresh".into(),
        args: None,
    })
    .unwrap();
    assert!(command.get("module_id").is_some(), "serde écrit module_id");
    assert!(command.get("moduleId").is_none(), "et surtout pas moduleId");
    let command_variant = variants
        .iter()
        .find(|v| v["name"] == json!("command"))
        .expect("variante command");
    assert!(command_variant["fields"].get("module_id").is_some());
    assert!(command_variant["fields"].get("moduleId").is_none());
    assert_eq!(command_variant["fields"]["args"]["optional"], json!(true));

    // The two enums that rename by hand, and the one that follows the convention.
    assert_eq!(
        serde_json::to_value(TemperatureUnit::Celsius).unwrap(),
        json!("C")
    );
    assert_eq!(
        types["TemperatureUnit"]["variants"][0]["name"],
        json!("C"),
        "un rename explicite doit gagner sur la convention"
    );
    assert_eq!(
        serde_json::to_value(MapInteractionMode::PanZoom).unwrap(),
        json!("pan-zoom")
    );
    assert_eq!(
        serde_json::to_value(Tone::Primary).unwrap(),
        json!("primary")
    );
}
