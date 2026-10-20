//! A module's data from before its resource is taken over once: on whatever path comes first,
//! never again, and never over what the resource already holds.

use std::sync::atomic::{AtomicUsize, Ordering};

use chrono::{TimeZone, Utc};
use portaki_sdk::contracts::i18n::I18nText;
use portaki_sdk::resource::{Actor, Entry, HistoryEvent, HistoryLine};
use portaki_test_utils::MockContext;
use serde_json::json;
use uuid::Uuid;

#[portaki_sdk::item(key = "title")]
pub struct Rule {
    #[field(required, label = "host.rule.title")]
    pub title: I18nText,
}

#[portaki_sdk::item(meta)]
pub struct Display {
    #[field(between(2, 6), label = "host.cardLimit.label")]
    pub card_limit: u32,
}

static READS: AtomicUsize = AtomicUsize::new(0);

/// What the module stored before: two rules and a card limit.
fn earlier_rules() -> portaki_sdk::Result<Option<(Vec<Rule>, Display)>> {
    READS.fetch_add(1, Ordering::SeqCst);
    let rule = |id: &str, fr: &str| Rule {
        id: id.into(),
        title: I18nText::new(fr, ""),
    };
    Ok(Some((
        vec![rule("a", "Calme après 22 h"), rule("b", "Pas de fête")],
        Display { card_limit: 3 },
    )))
}

#[portaki_sdk::collection(name = "rules", item = Rule, meta = Display, legacy = earlier_rules, import)]
pub struct Rules;

#[test]
fn the_first_read_takes_the_earlier_data_over_once() {
    MockContext::host().run(|ctx| {
        let mut rules = Rules::load(&ctx).unwrap();
        assert_eq!(rules.len(), 2);
        assert_eq!(rules.meta().card_limit, 3);
        assert!(READS.load(Ordering::SeqCst) >= 1);
        // The host removes one: the earlier data is not read back over it.
        rules.rows_mut().pop();
        Rules::save(&rules).unwrap();
        assert_eq!(Rules::load(&ctx).unwrap().len(), 1, "taken over once");
    });
}

#[test]
fn a_host_save_before_any_read_keeps_the_earlier_rows() {
    MockContext::host().run(|ctx| {
        // The form sends the settings only: the rows adopted stay, the limit changes.
        __portaki_rules_update_config(ctx.clone(), json!({ "card_limit": 5 })).unwrap();
        let rules = Rules::load(&ctx).unwrap();
        assert_eq!(rules.len(), 2);
        assert_eq!(rules.meta().card_limit, 5);
    });
}

#[test]
fn an_import_before_any_read_adds_to_the_earlier_rows() {
    MockContext::host().run(|ctx| {
        let args = serde_json::from_value(
            json!({ "resource": "rules", "rows": [ { "title": "calme après 22 h" }, { "title": "Animaux" } ] }),
        )
        .unwrap();
        let report = __portaki_rules_import_rows(ctx.clone(), args).unwrap();
        assert_eq!((report.added, report.skipped), (1, 1));
        assert_eq!(Rules::load(&ctx).unwrap().len(), 3);
    });
}

#[test]
fn what_the_resource_holds_is_never_overwritten() {
    let stored = json!([ { "id": "x", "title": { "fr": "Déjà là" } } ]);
    MockContext::host()
        .with_resource("rules", stored, json!({}))
        .run(|ctx| {
            let rules = Rules::load(&ctx).unwrap();
            assert_eq!(rules.len(), 1);
            assert_eq!(rules.rows()[0].title.get("fr"), "Déjà là");
        });
}

// --- Records ---------------------------------------------------------------------------------

#[portaki_sdk::choice(label = "status")]
pub enum Status {
    #[initial]
    New,
    #[done]
    Resolved,
}

#[portaki_sdk::item(key = "summary")]
pub struct Report {
    #[field(required, label = "form.summary.label")]
    pub summary: String,
}

const EARLIER: u128 = 0x5eed;

fn earlier_reports() -> portaki_sdk::Result<Vec<Entry<Report, Status>>> {
    let at = Utc.with_ymd_and_hms(2026, 6, 2, 9, 0, 0).unwrap();
    Ok(vec![Entry {
        id: Uuid::from_u128(EARLIER),
        item: Report {
            id: String::new(),
            summary: "Fuite sous l'évier".into(),
        },
        status: Status::Resolved,
        stay_id: Some(Uuid::from_u128(7)),
        created_at: at,
        actor: Actor::Guest,
        guest_name: "Marie".into(),
        check_in: None,
        check_out: None,
        history: vec![HistoryLine {
            at,
            actor: Actor::Guest,
            event: HistoryEvent::Created,
        }],
    }])
}

#[portaki_sdk::record(name = "reports", item = Report, status = Status, legacy = earlier_reports)]
pub struct Reports;

#[test]
fn earlier_records_come_first_and_a_new_one_joins_them() {
    let (ctx, host) = MockContext::guest().build();
    portaki_sdk::host::with_host(host, ctx.clone(), || {
        // The first thing that happens is a new deposit: the earlier one is taken over before it.
        __portaki_reports_submit(ctx.clone(), json!({ "summary": "Plus d'eau chaude" })).unwrap();
        let all = Reports::list(&ctx).unwrap();
        assert_eq!(all.len(), 2);
        let earlier = all
            .iter()
            .find(|e| e.id == Uuid::from_u128(EARLIER))
            .unwrap();
        assert_eq!(earlier.status, Status::Resolved);
        assert_eq!(earlier.guest_name, "Marie");
        // Read again: still two, the earlier one not stored a second time.
        assert_eq!(Reports::list(&ctx).unwrap().len(), 2);
    });
}
