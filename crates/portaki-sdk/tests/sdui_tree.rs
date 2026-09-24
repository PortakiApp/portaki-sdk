//! Le parcours de l'arbre SDUI (`Component::child_nodes`, `type_name`, `SduiPrimitive`).
//!
//! Tout est généré depuis `sdui_primitives.json` : ces tests relisent le contrat et vérifient que
//! chaque primitif y est couvert, pour qu'un primitif ajouté au JSON ne puisse pas échapper au
//! parcours des surfaces dans les tests des modules.

use std::fs;
use std::path::PathBuf;

use portaki_sdk::sdui::primitives::{
    Accordion, Card, Component, SduiPrimitive, Split, Stack, SurfacePrimitive, Tabs, Text,
    NODE_ITEM_TYPES,
};
use portaki_sdk::sdui::{AccordionItem, TabItem};
use serde_json::{json, Value};
use syn::{Fields, Item};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn contract_json() -> Vec<Value> {
    let raw = fs::read_to_string(manifest_dir().join("sdui_primitives.json")).expect("contrat");
    serde_json::from_str(&raw).expect("JSON du contrat")
}

/// `(serde_name, has_children)` de chaque primitif, dans l'ordre du contrat.
fn contract() -> Vec<(String, bool)> {
    contract_json()
        .iter()
        .map(|p| {
            (
                p["serde_name"].as_str().expect("serde_name").to_string(),
                p["has_children"].as_bool().expect("has_children"),
            )
        })
        .collect()
}

#[test]
fn type_names_list_every_primitive_of_the_contract() {
    let names: Vec<String> = contract().into_iter().map(|(name, _)| name).collect();
    assert_eq!(Component::TYPE_NAMES, names.as_slice());

    let variants: Vec<String> = contract_json()
        .iter()
        .map(|p| p["name"].as_str().expect("name").to_string())
        .collect();
    assert_eq!(Component::VARIANT_NAMES, variants.as_slice());
}

#[test]
fn every_primitive_walks_its_children_and_names_itself() {
    for (name, has_children) in contract() {
        let node: Component = serde_json::from_value(json!({
            "type": name,
            "children": [{"type": "Text"}, {"type": "Divider"}],
        }))
        .unwrap_or_else(|e| panic!("{name} : {e}"));

        assert_eq!(node.type_name(), name);
        let expected = if has_children { 2 } else { 0 };
        assert_eq!(node.child_nodes().len(), expected, "enfants de {name}");
    }
}

#[test]
fn a_single_node_field_is_a_child() {
    let split: Component = Split::new()
        .left(Text::new().text("gauche"))
        .right(Card::new())
        .child(Stack::new())
        .into();

    let types: Vec<&str> = split.child_nodes().iter().map(|n| n.type_name()).collect();
    assert_eq!(types, ["Text", "Card", "Stack"]);
}

#[test]
fn accordion_and_tab_contents_are_children() {
    let accordion: Component = Accordion::new()
        .items(vec![
            AccordionItem {
                title: Some("Arrivée".into()),
                content: Some(Box::new(Card::new().into())),
            },
            AccordionItem::default(),
        ])
        .into();
    let tabs: Component = Tabs::new()
        .tabs(vec![TabItem {
            content: Some(Box::new(Text::new().into())),
            ..TabItem::default()
        }])
        .into();

    assert_eq!(accordion.child_nodes()[0].type_name(), "Card");
    assert_eq!(accordion.child_nodes().len(), 1);
    assert_eq!(tabs.child_nodes()[0].type_name(), "Text");
}

#[test]
fn sdui_primitive_downcasts_its_own_variant_only() {
    let card: Component = Card::new().title("Wi-Fi").into();

    assert_eq!(
        Card::from_component(&card).and_then(|c| c.title.as_deref()),
        Some("Wi-Fi")
    );
    assert!(Text::from_component(&card).is_none());
    assert_eq!(SurfacePrimitive::TYPE_NAME, "Surface");
    assert_eq!(
        Component::from(SurfacePrimitive::new()).variant_name(),
        "SurfacePrimitive"
    );
}

/// `build.rs` ne voit pas l'intérieur des types de `common.rs` : il ne suit que le `content` des
/// types qu'il liste. Un nouveau type qui porte un `Component` doit y être ajouté.
#[test]
fn every_common_type_holding_a_node_is_walked() {
    let source = fs::read_to_string(manifest_dir().join("src/sdui/common.rs")).expect("common.rs");
    let file = syn::parse_file(&source).expect("parse common.rs");

    let mut holders = Vec::new();
    for item in file.items {
        let Item::Struct(item) = item else { continue };
        let Fields::Named(fields) = &item.fields else {
            continue;
        };
        for field in &fields.named {
            let ty = quote::ToTokens::to_token_stream(&field.ty).to_string();
            if ty.contains("Component") {
                let name = field.ident.as_ref().expect("champ nommé").to_string();
                holders.push(format!("{}.{name}", item.ident));
            }
        }
    }

    let walked: Vec<String> = NODE_ITEM_TYPES
        .iter()
        .map(|ty| format!("{ty}.content"))
        .collect();
    assert_eq!(holders, walked);
}

/// A chart is one leaf, whatever its length: its data rides in `points`, never in children.
#[test]
fn an_image_can_ask_for_a_thumbnail() {
    use portaki_sdk::sdui::primitives::Image;
    use portaki_sdk::sdui::ImageSize;

    let json = serde_json::to_value(Image::new().url("portaki-file:x").size(ImageSize::Thumb))
        .expect("image json");
    assert_eq!(json["size"], "thumb");
}

#[test]
fn chart_is_a_single_leaf_node() {
    use portaki_sdk::sdui::primitives::Chart;
    use portaki_sdk::sdui::{ChartKind, ChartPoint, Swatch};

    let chart: Component = Chart::new()
        .kind(ChartKind::HorizontalBars)
        .swatch(Swatch::Red)
        .points(vec![
            ChartPoint::new("i18n:cat.appliance", 2.0).display("2 signalements"),
            ChartPoint::new("i18n:cat.access", 1.0),
        ])
        .into();
    assert!(chart.child_nodes().is_empty());
    assert_eq!(
        serde_json::to_value(&chart).unwrap(),
        json!({
            "type": "Chart",
            "kind": "horizontal_bars",
            "swatch": "red",
            "points": [
                { "label": "i18n:cat.appliance", "value": 2.0, "display": "2 signalements" },
                { "label": "i18n:cat.access", "value": 1.0 }
            ]
        })
    );
}

/// The two leaves the module stats and checklist editors need: rows ride in fields, not children.
#[test]
fn editable_list_and_feed_item_are_leaves_on_the_wire() {
    use portaki_sdk::sdui::primitives::{EditableList, FeedItem};
    use portaki_sdk::sdui::{EditableListItem, FeedStatus, Tone};

    let list: Component = EditableList::new()
        .name("items")
        .photoToggle(true)
        .addLabel("Ajouter une tâche")
        .items(vec![EditableListItem {
            photo: Some(true),
            ..EditableListItem::new("Photo du salon")
        }])
        .into();
    assert!(list.child_nodes().is_empty());
    assert_eq!(
        serde_json::to_value(&list).unwrap(),
        json!({
            "type": "EditableList",
            "name": "items",
            "photoToggle": true,
            "addLabel": "Ajouter une tâche",
            "items": [{ "label": "Photo du salon", "photo": true }]
        })
    );

    let row: Component = FeedItem::new()
        .title("Fuite sous l'évier")
        .dotTone(Tone::Warning)
        .status(FeedStatus::new("En cours", Tone::Warning))
        .into();
    assert_eq!(
        serde_json::to_value(&row).unwrap(),
        json!({
            "type": "FeedItem",
            "title": "Fuite sous l'évier",
            "dotTone": "warning",
            "status": { "label": "En cours", "tone": "warning" }
        })
    );
}
