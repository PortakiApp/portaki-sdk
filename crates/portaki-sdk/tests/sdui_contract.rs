//! What the contract must carry for the shells that render it.
//!
//! The guest booklet kept a second, hand-written copy of this contract and read thirty-three
//! (primitive, field) pairs it did not declare. A mismatched name breaks nothing — it renders
//! nothing — so the drift went unseen until the types were generated from the contract.
//!
//! Eighteen of those fields are now here. These tests pin them, because the only thing that made
//! them invisible was that nobody compared the two.

use portaki_sdk::sdui::primitives::{
    Accordion, ActionRow, Anchor, BackButton, BottomTabBar, BulletList, ColorDotItem, Component,
    DateColumn, Dot, FilterBar, Form, IconButton, Map, Skeleton, Split, Tabs, Text, TextArea,
    TimeColumn,
};
use portaki_sdk::sdui::Action;
use portaki_sdk::sdui::{
    AccordionItem, ActionRowItem, FilterBarChip, MapClustering, TabBarItem, TabItem,
};

fn wire(component: Component) -> serde_json::Value {
    serde_json::to_value(component).expect("serialise")
}

#[test]
fn a_nested_node_travels_inside_a_field() {
    let split = Split::new().left(Text::new().text("gauche")).ratio(0.3);
    let json = wire(split.into());

    assert_eq!(json["left"]["type"], "Text");
    assert_eq!(json["left"]["text"], "gauche");
    assert_eq!(json["ratio"], 0.3);
}

#[test]
fn an_accordion_carries_its_sections() {
    let json = wire(
        Accordion::new()
            .items(vec![AccordionItem {
                title: Some("Arrivée".into()),
                content: Some(Box::new(Text::new().text("code 1234").into())),
            }])
            .into(),
    );

    assert_eq!(json["items"][0]["title"], "Arrivée");
    assert_eq!(json["items"][0]["content"]["type"], "Text");
}

#[test]
fn tabs_carry_their_panels() {
    let json = wire(
        Tabs::new()
            .tabs(vec![TabItem {
                id: Some("wifi".into()),
                label: Some("Wi-Fi".into()),
                content: Some(Box::new(Text::new().text("mot de passe").into())),
            }])
            .into(),
    );

    assert_eq!(json["tabs"][0]["id"], "wifi");
    assert_eq!(json["tabs"][0]["content"]["type"], "Text");
}

/// `tabs` and `clustering` were untyped `Value`: every shell read a shape nothing declared.
#[test]
fn the_formerly_untyped_fields_now_have_a_shape() {
    let bar = wire(
        BottomTabBar::new()
            .tabs(vec![TabBarItem {
                id: Some("home".into()),
                label: Some("Accueil".into()),
                ..Default::default()
            }])
            .activeTab("home")
            .into(),
    );
    assert_eq!(bar["tabs"][0]["id"], "home");
    assert_eq!(bar["activeTab"], "home");

    let map = wire(
        Map::new()
            .clustering(MapClustering {
                enabled: true,
                radius: Some(40.0),
                ..Default::default()
            })
            .into(),
    );
    assert_eq!(map["clustering"]["enabled"], true);
    assert_eq!(map["clustering"]["radius"], 40.0);
}

/// A column shows ONE date with a label; `dates`/`times` never matched what the shells render.
#[test]
fn the_columns_declare_the_single_value_they_show() {
    let date = wire(DateColumn::new().date("12 sept.").label("Arrivée").into());
    assert_eq!(date["date"], "12 sept.");
    assert_eq!(date["label"], "Arrivée");

    let time = wire(TimeColumn::new().time("15:00").label("Après").into());
    assert_eq!(time["time"], "15:00");
    assert_eq!(time["label"], "Après");
}

/// An icon-only button needs a name, and a form needs somewhere to send itself.
#[test]
fn the_buttons_and_the_form_declare_what_they_need() {
    assert_eq!(
        wire(IconButton::new().label("Fermer").into())["label"],
        "Fermer"
    );
    assert_eq!(
        wire(BackButton::new().label("Retour").into())["label"],
        "Retour"
    );
    assert!(wire(Form::new().onSubmit(Action::copy("x", None)).into())["onSubmit"].is_object());
}

#[test]
fn the_scalars_the_booklet_reads_are_declared() {
    assert_eq!(wire(Dot::new().pulse(true).into())["pulse"], true);
    assert_eq!(wire(TextArea::new().rows(4).into())["rows"], 4);
    assert_eq!(
        wire(ColorDotItem::new().subtitle("gris perle").into())["subtitle"],
        "gris perle"
    );

    let skeleton = wire(Skeleton::new().width(120.0).height(16.0).into());
    assert_eq!(skeleton["width"], 120.0);
    assert_eq!(skeleton["height"], 16.0);
}

#[test]
fn the_collections_the_booklet_reads_are_declared() {
    let bullets = wire(
        BulletList::new()
            .items(vec!["un".into(), "deux".into()])
            .into(),
    );
    assert_eq!(bullets["items"][1], "deux");

    let filters = wire(
        FilterBar::new()
            .filters(vec![FilterBarChip {
                id: Some("wifi".into()),
                label: Some("Wi-Fi".into()),
            }])
            .into(),
    );
    assert_eq!(filters["filters"][0]["label"], "Wi-Fi");

    let actions = wire(
        ActionRow::new()
            .actions(vec![ActionRowItem {
                label: Some("Ouvrir".into()),
                action: None,
            }])
            .into(),
    );
    assert_eq!(actions["actions"][0]["label"], "Ouvrir");
}

/// An anchor wraps what it anchors; it declared no children, so a shell rendering them was
/// rendering something the contract denied.
#[test]
fn an_anchor_declares_the_children_it_wraps() {
    let json = wire(
        Anchor::new()
            .targetId("wifi")
            .child(Text::new().text("Wi-Fi"))
            .into(),
    );

    assert_eq!(json["targetId"], "wifi");
    assert_eq!(json["children"][0]["type"], "Text");
}

/// No synonyms. Twelve of the pairs were the booklet reading another name for a
/// field the contract already had — `Text.content` for `text`, `Quote.content` for `text`,
/// `CountdownTimer.targetIso` for `until`. Adding those would have given the contract two names
/// for one thing, so the booklet was corrected instead. This pins the absence.
///
/// `swatch` is deliberately NOT in this list: `Dot` and `ColorDotItem` both declare it beside
/// `color`, a named tint beside a free-form one. The shells were right to read it, and a stale
/// vendored copy of this contract is what made it look like drift.
#[test]
fn the_contract_grew_no_synonyms() {
    let contract: serde_json::Value =
        serde_json::from_str(include_str!("../sdui_primitives.json")).expect("contract");

    let banned = [
        ("Text", "content"),
        ("Quote", "content"),
        ("Eyebrow", "content"),
        ("Highlight", "content"),
        ("Code", "content"),
        ("CountdownTimer", "targetIso"),
        ("Hero", "illustrationId"),
        ("KeyValue", "label"),
        ("SectionListItem", "section"),
        ("FormStepper", "current"),
        ("Map", "static"),
        ("ColorDotItem", "colorRole"),
        ("DotIndicator", "active"), // → `index`, qui est le même nombre
        ("EmptyState", "message"),  // → `description`
    ];

    for (primitive, field) in banned {
        let entry = contract
            .as_array()
            .expect("array")
            .iter()
            .find(|n| n["name"] == primitive)
            .unwrap_or_else(|| panic!("{primitive} absent du contrat"));
        assert!(
            entry["fields"].get(field).is_none(),
            "{primitive}.{field} est un synonyme d'un champ existant — corrigez le rendu, \
             pas le contrat"
        );
    }
}
