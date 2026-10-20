//! Declared resources against the stateful mock repository: what `#[collection]` generates
//! keeps the host's work, adds imports without overwriting, and reads back typed.

use portaki_sdk::contracts::i18n::I18nText;
use portaki_sdk::resource::{Choice, Item};
use portaki_test_utils::MockContext;
use serde_json::json;

#[portaki_sdk::choice(label = "rule.status")]
pub enum Status {
    #[default]
    Neutral,
    #[icon(portaki_sdk::vocab::IconName::TriangleAlert)]
    Important,
}

#[portaki_sdk::item(key = "title")]
pub struct Rule {
    #[field(required, max_chars = 20, label = "host.rule.title")]
    pub title: I18nText,
    #[field(label = "host.rule.status")]
    pub status: Status,
    #[field(kind = "time_range", label = "host.rule.hours")]
    pub hours: String,
}

#[portaki_sdk::item(meta)]
pub struct Display {
    #[field(between(2, 6), label = "host.cardLimit.label")]
    pub card_limit: u32,
}

#[portaki_sdk::collection(name = "rules", item = Rule, meta = Display, max = 3, import, read)]
pub struct Rules;

#[test]
fn a_choice_knows_its_values_labels_and_icons() {
    assert_eq!(Status::ALL.len(), 2);
    assert_eq!(Status::Important.wire(), "important");
    assert_eq!(Status::Important.label_key(), "i18n:rule.status.important");
    assert!(Status::Important.icon().is_some());
    assert_eq!(Status::from_wire("neutral"), Some(Status::Neutral));
    assert_eq!(Rule::fields().len(), 3);
}

#[test]
fn the_host_saves_then_an_import_adds_without_touching_his_rows() {
    MockContext::host().with_locale("fr-FR").run(|ctx| {
        __portaki_rules_update_config(
            ctx.clone(),
            json!({ "rules": [ { "id": "a", "title": "Silence", "status": "important", "hours": "22:00-08:00" } ],
                    "card_limit": "4" }),
        )
        .unwrap();
        let report = __portaki_rules_import_rows(
            ctx.clone(),
            serde_json::from_value(json!({ "resource": "rules", "rows": [
                { "title": "silence" }, { "title": "Animaux bienvenus", "status": "neutral" }
            ] }))
            .unwrap(),
        )
        .unwrap();
        assert_eq!((report.added, report.skipped), (1, 1));

        let rules = Rules::load(&ctx).unwrap();
        assert_eq!(rules.len(), 2);
        assert_eq!(rules.rows()[0].id, "a");
        assert_eq!(rules.rows()[0].title.get("fr"), "Silence");
        assert_eq!(rules.rows()[0].status, Status::Important);
        assert_eq!(rules.rows()[0].hours, "22:00-08:00");
        assert_eq!(rules.meta().card_limit, 4);
        assert_eq!(rules.rows()[1].title.get("fr"), "Animaux bienvenus");

        // An English host edits the first rule: the French stays.
        let en = ctx.clone();
        let mut en = en;
        en.locale = "en-US".into();
        __portaki_rules_update_config(en, json!({ "rules": [ { "id": "a", "title": "Quiet" }, { "id": rules.rows()[1].id } ] }))
            .unwrap();
        let rules = Rules::load(&ctx).unwrap();
        assert_eq!(rules.rows()[0].title.get("fr"), "Silence");
        assert_eq!(rules.rows()[0].title.get("en"), "Quiet");
        assert_eq!(rules.len(), 2);
    });
}

#[test]
fn read_rows_gives_the_declared_fields_only() {
    MockContext::host()
        .with_rows(
            "PortakiResource",
            vec![json!({ "id": "00000000-0000-0000-0000-000000000001", "resource": "rules",
                "rows_json": r#"[{"id":"a","title":{"fr":"Pas de fête"},"status":"important","hours":"","note":"S3CR-undeclared"}]"#,
                "meta_json": r#"{"card_limit":4}"#, "updated_at": "2026-10-19T00:00:00Z" })],
        )
        .run(|ctx| {
            let read = __portaki_rules_read_rows(
                ctx.clone(),
                serde_json::from_value(json!({ "resource": "rules" })).unwrap(),
            )
            .unwrap();
            assert_eq!(
                read,
                json!({ "resource": "rules", "rows": [
                    { "title": { "fr": "Pas de fête" }, "status": "important", "hours": "" }
                ] })
            );
            let wrong = __portaki_rules_read_rows(
                ctx,
                serde_json::from_value(json!({ "resource": "devices" })).unwrap(),
            );
            assert!(wrong.unwrap_err().to_string().contains("resource_unknown"));
        });
}

#[test]
fn problems_block_publication_and_name_the_row() {
    MockContext::host().run(|ctx| {
        __portaki_rules_update_config(
            ctx.clone(),
            json!({ "rules": [ { "id": "a", "title": "Une règle bien trop longue pour la carte" } ], "card_limit": 9 }),
        )
        .unwrap();
        let readiness = __portaki_rules_publish_readiness(ctx).unwrap();
        let ids: Vec<&str> = readiness.items.iter().map(|i| i.id.as_str()).collect();
        assert!(ids.contains(&"resources.rules.a.title"), "{ids:?}");
        assert!(ids.contains(&"resources.rules.card_limit"), "{ids:?}");
    });
}

#[test]
fn the_editor_follows_the_dashboard_wire_format() {
    MockContext::host()
        .with_rows(
            "PortakiResource",
            vec![json!({ "id": "00000000-0000-0000-0000-000000000001", "resource": "rules",
                "rows_json": r#"[{"id":"a","title":{"fr":"Silence"},"status":"important","hours":""}]"#,
                "meta_json": "{}", "updated_at": "2026-10-19T00:00:00Z" })],
        )
        .run(|ctx| {
            let rules = Rules::load(&ctx).unwrap();
            let editor = portaki_sdk::resource::CollectionEditor::new(&ctx, &rules)
                .settings("host.display.title", Vec::new())
                .build();
            let tree = serde_json::to_value(&editor).unwrap();
            let text = tree.to_string();
            assert!(text.contains(r#""rowName":"rules""#), "{text}");
            assert!(text.contains(r#""name":"rules.0.title""#), "{text}");
            assert!(text.contains(r#""name":"card_limit""#), "{text}");
            assert!(text.contains("rulesNew"), "{text}");
        });
}
