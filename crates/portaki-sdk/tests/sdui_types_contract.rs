//! Dérive `contracts/sdui_types.json` depuis le Rust, et le fige.
//!
//! `sdui_primitives.json` nomme les types de ses champs — `"action": "Action"` — sans jamais les
//! décrire. Un consommateur du contrat peut donc afficher un primitif mais pas éditer une `Action`
//! autrement qu'en JSON brut : il ignore ses variantes. Ce document comble ce trou.
//!
//! Il est **dérivé, pas écrit** : les définitions vivent dans `sdui/common.rs` et `sdui/action.rs`,
//! et c'est de là qu'on les lit. Une copie tenue à la main aurait dérivé — le dashboard l'a déjà
//! payé sur ses six champs communs.
//!
//! Regénérer après avoir touché un type : `BLESS=1 cargo test -p portaki-sdk --test
//! sdui_types_contract`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

use serde_json::{json, Map, Value};
use syn::{Attribute, Expr, Fields, Item, Lit, Type};

/// Les fichiers où vivent les types nommés par le contrat.
const SOURCES: [&str; 2] = ["src/sdui/common.rs", "src/sdui/action.rs"];

/// Ce que `build.rs` pose sur chaque primitive sans que le contrat le dise.
///
/// Leur type est référencé par tous les primitifs et par aucun champ déclaré : sans cette racine,
/// la fermeture transitive les manquerait et un consommateur n'aurait pas de quoi éditer `tone`.
const COMMON_FIELD_TYPES: [&str; 5] = [
    "Tone",
    "Emphasis",
    "SurfaceLevel",
    "Animation",
    "Visibility",
];

/// Les types que le contrat traite comme des scalaires : ils n'ont rien à décrire.
fn is_scalar(ty: &str) -> bool {
    matches!(
        ty,
        "String" | "bool" | "f64" | "u32" | "i64" | "u64" | "usize" | "Value" | "Component"
    )
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

// ── Lecture des attributs serde ──────────────────────────────────────────────

/// `#[serde(rename = "…")]` porté par une variante ou un champ.
fn serde_rename(attrs: &[Attribute]) -> Option<String> {
    serde_string_attr(attrs, "rename")
}

/// `#[serde(rename_all = "…")]` porté par le conteneur.
fn serde_rename_all(attrs: &[Attribute]) -> Option<String> {
    serde_string_attr(attrs, "rename_all")
}

/// `#[serde(tag = "…")]` — une énumération taguée en interne est plate sur le fil.
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
                // `alias`, `skip_serializing_if`, `default`… : consommer la valeur éventuelle
                // pour que l'analyse ne s'arrête pas au premier voisin.
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

/// Vrai quand `#[portaki_sdk_macros::wire]` couvre l'item.
///
/// Le macro ajoute `rename_all = "camelCase"` **quand le conteneur n'en porte pas déjà un**. Son
/// effet est donc invisible dans le source, et l'ignorer donnerait des noms de champs en
/// `snake_case` que personne n'émet.
fn has_wire(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        let path = attr.path();
        path.segments
            .last()
            .map(|s| s.ident == "wire")
            .unwrap_or(false)
    })
}

/// La convention effective du conteneur, macro comprise.
///
/// Sur une énumération, elle nomme les **variantes** — pas les champs de leurs variantes. Cette
/// distinction est celle de serde, et elle se voit à l'œil nu sur `Action` : la variante s'écrit
/// `openOverlay`, son champ reste `surface_render`.
fn effective_rename_all(attrs: &[Attribute]) -> Option<String> {
    serde_rename_all(attrs).or_else(|| has_wire(attrs).then(|| "camelCase".to_string()))
}

/// La convention des champs **portés par une variante**.
///
/// Renommer les variantes ne renomme pas leurs champs : il faut `rename_all_fields` sur
/// l'énumération, ou `rename_all` sur la variante. Sans l'un des deux, serde écrit l'identifiant
/// Rust tel quel — et `wire` n'y change rien, puisqu'il ne pose que `rename_all`.
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
        // Sans convention, serde écrit l'identifiant tel quel.
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

// ── Lecture des types ────────────────────────────────────────────────────────

/// Le nom du type porté par un champ, `Option` et `Vec` retirés.
///
/// Retourne aussi si le champ est facultatif : c'est ce qui distingue un champ qu'un éditeur peut
/// omettre d'un champ qu'il doit demander.
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
    // `Option<Box<Component>>` : le `Box` se trouve sous l'enveloppe, pas devant elle.
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

/// Le type rendu sans espaces, sans `Box`, et réduit à son dernier segment de chemin.
///
/// `Box` n'existe que pour donner une taille finie à un primitif qui en contient un autre
/// (`build.rs` l'ajoute) : il ne dit rien du fil, et le contrat ne le nomme pas.
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

/// `crate::sdui::common::Tone` → `Tone`, en laissant intacts les paramètres génériques.
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
    /// Les types nommés par un champ mais jamais définis — attendus scalaires, sinon c'est un trou.
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
                    // `pub struct VisibilityExpr(pub String);` — serde le sérialise de façon
                    // transparente, donc sur le fil c'est une chaîne. Le dire évite qu'un
                    // consommateur cherche une structure qui n'apparaît jamais.
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
    catalog
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

// ── Fermeture transitive depuis les primitives ───────────────────────────────

/// Les types qu'un consommateur du contrat peut rencontrer, et eux seuls.
///
/// Part des champs déclarés par `sdui_primitives.json` et des champs communs, puis suit les
/// références. `Action` seule tire `OverlayPresentation` et `OverlayArgs` : publier la racine sans
/// sa descendance laisserait l'éditeur à mi-chemin.
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
            // Un newtype porte son type au même niveau que son `kind`.
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

/// Ce que le générateur affirme du fil doit être ce que serde en fait.
///
/// La convention de nommage est **rejouée** ici à partir des attributs, `wire` compris ; sans ce
/// test elle serait une croyance. On sérialise donc de vraies valeurs et on compare.
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

    // Le piège que ce test existe pour attraper : `rename_all` sur une énumération nomme ses
    // variantes et **pas** les champs qu'elles portent. `command` est en camelCase, `module_id`
    // reste en snake_case. Un contrat qui annoncerait `moduleId` ferait écrire aux consommateurs
    // un champ que le Rust ne relit pas.
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

    // Les deux enums qui renomment à la main, et celui qui suit la convention.
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
