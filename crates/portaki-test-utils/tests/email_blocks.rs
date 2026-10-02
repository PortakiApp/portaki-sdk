//! A module that declares its stay-email blocks with `#[portaki_sdk::email_blocks]`.
//!
//! The point of this file is that it **compiles**: the macro's per-pair `const` assertion only
//! bites in a real crate, and a test on the generated tokens would not catch a `BlockType` whose
//! `renders_in` stopped being usable in `const` context. What follows then drives the generated
//! `emailContext` the way the platform does.

mod common;

use portaki_sdk::email::{BlockTone, EmailBlock, EmailBlocks, EmailContextArgs, EmailTemplateKey};
use portaki_sdk::prelude::*;
use portaki_sdk::wasm::registry::{declarations, HandlerKind};
use portaki_test_utils::MockContext;
use serde_json::json;

#[portaki_sdk::config]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    #[field(label = "config.spot")]
    pub spot: String,
    /// A test switch: answer a kind of block the module did not declare.
    pub stray: bool,
    /// A test switch: answer a text the platform would have to shorten.
    pub rambling: bool,
}

#[portaki_sdk::email_blocks(
    Arrival | EmailTemplateKey::ArrivalDay => [BlockType::Pairs],
    PostArrival => [Info],
)]
pub fn email_blocks(ctx: Context, _args: EmailContextArgs) -> Result<EmailBlocks> {
    let config = Config::load(&ctx)?;
    let mut blocks = EmailBlocks::new()
        .with(
            EmailBlock::pairs("Parking")
                .title("Votre place")
                .row("Place", config.spot)
                .link("Voir le plan", "ev-parking"),
        )
        .with(EmailBlock::info(
            "Recharge",
            if config.rambling {
                "x".repeat(200)
            } else {
                "La borne est au niveau -1.".to_string()
            },
        ));
    if config.stray {
        blocks.push(EmailBlock::alert(
            "Travaux",
            "La rue est fermée.",
            BlockTone::Warning,
        ));
    }
    Ok(blocks)
}

fn spot() -> Config {
    Config {
        spot: "n° 14".into(),
        stray: false,
        rambling: false,
    }
}

fn email_context(config: Config, args: serde_json::Value) -> Result<serde_json::Value> {
    let declaration = declarations()
        .find(|d| d.kind == HandlerKind::Query && d.name == "emailContext")
        .expect("#[email_blocks] generates emailContext");
    let (ctx, host) = MockContext::host().with_config(&config).build();
    portaki_sdk::host::with_host(host, ctx.clone(), || (declaration.dispatch)(ctx, args))
}

#[test]
fn the_platform_gets_the_blocks_declared_for_its_email() {
    let answer = email_context(spot(), json!({ "templateKey": "arrival" })).unwrap();

    assert_eq!(
        answer,
        json!({ "blocks": [{
            "type": "pairs",
            "label": { "fr": "Parking", "en": "Parking" },
            "title": { "fr": "Votre place", "en": "Votre place" },
            "rows": [{
                "label": { "fr": "Place", "en": "Place" },
                "value": { "fr": "n° 14", "en": "n° 14" },
            }],
            "linkLabel": { "fr": "Voir le plan", "en": "Voir le plan" },
            "anchor": "ev-parking",
        }]})
    );
}

#[test]
fn an_undeclared_email_or_none_is_not_asked() {
    for args in [json!({ "templateKey": "stay-link" }), json!({})] {
        assert_eq!(email_context(spot(), args).unwrap(), json!({}));
    }
}

/// `Info` est déclaré pour `post-arrival` seulement : il ne part pas dans l'arrivée, et
/// réciproquement.
#[test]
fn a_kind_goes_only_to_the_emails_it_is_declared_for() {
    let arrival = email_context(spot(), json!({ "templateKey": "arrival" })).unwrap();
    let post = email_context(spot(), json!({ "templateKey": "post-arrival" })).unwrap();

    let kinds = |answer: &serde_json::Value| {
        answer["blocks"]
            .as_array()
            .unwrap()
            .iter()
            .map(|b| b["type"].as_str().unwrap().to_string())
            .collect::<Vec<_>>()
    };
    assert_eq!(kinds(&arrival), ["pairs"]);
    assert_eq!(kinds(&post), ["info"]);
}

#[test]
fn a_kind_declared_nowhere_is_an_error() {
    let mut config = spot();
    config.stray = true;

    let error = email_context(config, json!({ "templateKey": "arrival" })).unwrap_err();

    assert!(
        error.to_string().contains("email_block_undeclared"),
        "{error}"
    );
    assert!(error.to_string().contains("alert"), "{error}");
}

/// La plateforme couperait sans rien dire ; ici le module l'apprend à `cargo test`.
#[test]
fn a_text_the_platform_would_shorten_is_an_error() {
    let mut config = spot();
    config.rambling = true;

    let error = email_context(config, json!({ "templateKey": "post-arrival" })).unwrap_err();

    assert!(
        error.to_string().contains("email_block_too_long"),
        "{error}"
    );
}

#[test]
fn the_catalogue_says_where_each_kind_renders() {
    assert!(BlockType::Pairs.renders_in(EmailTemplateKey::Arrival));
    assert!(!BlockType::Pairs.renders_in(EmailTemplateKey::StayLink));
    for kind in BlockType::ALL {
        assert!(!kind.templates().is_empty(), "{kind}");
        assert!(kind.templates().iter().all(|t| t.is_guest_stay()), "{kind}");
    }
}
