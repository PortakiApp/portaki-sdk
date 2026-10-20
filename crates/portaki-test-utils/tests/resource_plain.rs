//! A collection with no settings and a formatted text: it reads from an empty store, and a row
//! whose document has no words is a blank row.

use portaki_sdk::contracts::i18n::I18nText;
use portaki_sdk::resource::Item;
use portaki_test_utils::MockContext;
use serde_json::json;

#[portaki_sdk::item(key = "title")]
pub struct Section {
    #[field(label = "host.title.label")]
    pub title: I18nText,
    #[field(kind = "rich_text", label = "host.body.label")]
    pub body: I18nText,
}

#[portaki_sdk::collection(name = "sections", item = Section, import)]
pub struct Sections;

const EMPTY_DOC: &str = r#"{"type":"doc","content":[{"type":"paragraph"}]}"#;

#[test]
fn no_settings_reads_from_an_empty_store_and_back() {
    MockContext::host().run(|ctx| {
        assert!(Sections::load(&ctx).unwrap().is_empty());
        __portaki_sections_update_config(
            ctx.clone(),
            json!({ "sections": [ { "title": "Bienvenue", "body": "Le café est au placard." } ] }),
        )
        .unwrap();
        let sections = Sections::load(&ctx).unwrap();
        assert_eq!(sections.rows()[0].body.get("fr"), "Le café est au placard.");
    });
}

#[test]
fn a_document_without_words_is_a_blank_row() {
    MockContext::host().run(|ctx| {
        __portaki_sections_update_config(
            ctx.clone(),
            json!({ "sections": [ { "title": "", "body": EMPTY_DOC }, { "title": "Gardée", "body": EMPTY_DOC } ] }),
        )
        .unwrap();
        let sections = Sections::load(&ctx).unwrap();
        assert_eq!(sections.len(), 1, "the wordless row goes");
        assert_eq!(sections.rows()[0].title.get("fr"), "Gardée");
    });
}

#[test]
fn the_manifest_says_localized_formatted() {
    let schema = Section::fields()
        .iter()
        .find(|field| field.key == "body")
        .map(|field| field.kind);
    assert_eq!(schema, Some(portaki_sdk::resource::FieldKind::RichText));
}
