//! A module whose configuration holds translated text: one field, and list rows.
//!
//! The platform stores a `localized` value per language; the module reads it with
//! `I18nText::get` (guest) or `host_value` (host form), and the conformance battery holds the
//! manifest to what the code declares.

use portaki_sdk::contracts::i18n::I18nText;
use portaki_sdk::prelude::*;
use portaki_test_utils::MockContext;
use serde_json::json;

/// A step of the arrival: `id` identifies the row, the texts are translated.
#[portaki_sdk::params]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Step {
    pub id: String,
    pub title: I18nText,
    pub detail: Option<I18nText>,
    pub ends_at: Option<String>,
}

#[portaki_sdk::params]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Spot {
    pub slug: String,
    pub name: I18nText,
}

#[portaki_sdk::config]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    #[field(required, label = "config.welcome")]
    pub welcome: I18nText,
    #[field(label = "config.steps")]
    pub steps: Vec<Step>,
    #[field(label = "config.spots", item_id = "slug")]
    pub spots: Vec<Spot>,
}

/// What the host form shows for the welcome note and the first step.
#[portaki_sdk::query(name = "hostTexts")]
pub fn host_texts(ctx: Context) -> Result<Vec<String>> {
    let config = Config::load(&ctx)?;
    let mut texts = vec![config.welcome.host_value(&ctx).to_string()];
    texts.extend(
        config
            .steps
            .iter()
            .map(|s| s.title.host_value(&ctx).to_string()),
    );
    Ok(texts)
}

fn config() -> Config {
    Config {
        welcome: I18nText::new("Bienvenue", "Welcome").with("de", "Willkommen"),
        steps: vec![Step {
            id: "gate".into(),
            title: I18nText::new("Portail", "Gate"),
            detail: None,
            ends_at: Some("2026-10-01".into()),
        }],
        spots: Vec::new(),
    }
}

#[test]
fn the_host_form_shows_the_editor_language() {
    MockContext::host().with_config(&config()).run(|mut ctx| {
        assert_eq!(host_texts(ctx.clone()).unwrap(), ["Bienvenue", "Portail"]);
        ctx.locale = "de-DE".into();
        assert_eq!(host_texts(ctx.clone()).unwrap(), ["Willkommen", "Portail"]);
        ctx.locale = "en-GB".into();
        assert_eq!(host_texts(ctx).unwrap(), ["Welcome", "Gate"]);
    });
}

#[test]
fn a_config_round_trips_through_the_mock() {
    MockContext::host()
        .with_config(&config())
        .run(|ctx| assert_eq!(Config::load(&ctx).unwrap(), config()));
}

/// A config saved before the text was translated holds plain strings: every language reads it.
#[test]
fn a_legacy_string_reads_in_every_language() {
    MockContext::host()
        .with_config(&json!({
            "welcome": "Bienvenue",
            "steps": [{ "id": "gate", "title": { "en": "Gate" } }],
        }))
        .run(|ctx| {
            let config = Config::load(&ctx).unwrap();
            assert_eq!(config.welcome.get("en"), "Bienvenue");
            assert_eq!(config.steps[0].title.get("fr"), "Gate");
            assert!(config.steps[0].detail.is_none());
            assert!(!config.welcome.is_blank());
        });
}

portaki_test_utils::conformance!(
    dir = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/localized-config"
    )
);

/// What the battery checks, on a manifest that forgot the translated text.
#[test]
fn a_stale_manifest_is_caught() {
    let stale = portaki_test_utils::conformance::Module::at(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/localized-config-stale"
    ));
    let error = stale.check_manifest().unwrap_err().to_string();
    assert!(error.contains("`welcome` is an I18nText"), "{error}");
    assert!(
        error.contains("`spots` item.localized must be [\"name\"]"),
        "{error}"
    );
    assert!(
        error.contains("`nom`, which is not a field of Spot (slug, name)"),
        "{error}"
    );
}
