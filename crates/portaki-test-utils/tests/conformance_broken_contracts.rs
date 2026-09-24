//! Host surfaces whose typed answers are off contract, missing, or too slow fail `contracts`.
//!
//! The passing fixture's manifest declares a stats card and detail `reports` and a timeline task;
//! the handlers below break each promise.

mod common;

use std::time::Duration;

use common::{assert_reports, failing, passing};
use portaki_sdk::prelude::*;
use serde_json::{json, Value};

#[portaki_sdk::query(name = "statsSummary")]
pub fn stats_summary(_ctx: Context, args: Value) -> Result<Value> {
    if args["period"] == 365 {
        std::thread::sleep(Duration::from_millis(350));
    }
    Ok(json!({ "value": "1 234 567 890", "label": { "fr": "signalements" } }))
}

#[portaki_sdk::query(name = "timelineTasks")]
pub fn timeline_tasks(ctx: Context) -> Result<Value> {
    Ok(json!({ "tasks": [{
        "id": "cleaning:1",
        "at": "samedi matin",
        "propertyId": ctx.property_id,
        "title": { "fr": "Ménage", "en": "Cleaning" },
        "context": { "fr": "Après le départ", "en": "After check-out" },
        "items": []
    }] }))
}

#[portaki_sdk::query(name = "publishReadiness")]
pub fn publish_readiness(_ctx: Context) -> Result<Value> {
    Ok(json!({ "items": [{
        "id": "title", "level": "required", "ok": false,
        "label": { "fr": "Titre" }, "hint": { "fr": "Il manque le titre", "en": "Title missing" }
    }] }))
}

fn findings() -> portaki_test_utils::conformance::Findings {
    failing("contracts", passing().check_contracts())
}

#[test]
fn a_stats_summary_off_contract_is_reported() {
    let findings = findings();

    assert_reports(
        &findings,
        &["query `statsSummary`", "key `reports`", "/value"],
    );
    assert_reports(&findings, &["query `statsSummary`", "/label", "en"]);
}

#[test]
fn a_slow_stats_summary_is_reported() {
    let findings = findings();

    assert_reports(&findings, &["period 365", "at most 300 ms"]);
    assert!(
        !findings
            .problems()
            .iter()
            .any(|p| p.contains("period 30,") && p.contains("ms on the fixture")),
        "{findings}"
    );
}

#[test]
fn a_stats_detail_without_its_surface_is_reported() {
    assert_reports(
        &findings(),
        &[
            "property-stats-detail `reports`",
            "#[surface(host, id = \"reports\")]",
        ],
    );
}

#[test]
fn timeline_tasks_with_a_bad_date_or_no_items_are_reported() {
    let findings = findings();

    assert_reports(
        &findings,
        &["query `timelineTasks`", "/tasks/0/at", "samedi matin"],
    );
    assert_reports(&findings, &["query `timelineTasks`", "/tasks/0/items"]);
}

#[test]
fn a_publish_readiness_missing_a_language_is_reported() {
    assert_reports(
        &findings(),
        &["query `publishReadiness`", "/items/0/label", "en"],
    );
}
