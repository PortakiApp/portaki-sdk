//! What the Rust contract types serialize is valid against the JSON Schemas the dashboard and
//! the platform generate their types from.

use chrono::{TimeZone, Utc};
use portaki_sdk::contracts::i18n::I18nText;
use portaki_sdk::contracts::publish::{PublishCheck, PublishLevel, PublishReadiness};
use portaki_sdk::contracts::stats::{self, AttentionLevel, StatsSummaryArgs, TrendDirection};
use portaki_sdk::contracts::timeline::{
    self, TaskCompleteArgs, TaskToggleArgs, TaskUpdated, TimelineStay, TimelineTaskItem,
    TimelineTasks, TimelineTasksArgs,
};
use portaki_test_utils::conformance::{
    PUBLISH_READINESS_SCHEMA_V1, STATS_SUMMARY_SCHEMA_V1, TIMELINE_TASKS_SCHEMA_V1,
};
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

fn assert_valid(schema: &str, definition: &str, example: impl Serialize) {
    let mut schema: Value = serde_json::from_str(schema).unwrap();
    schema["$ref"] = Value::String(format!("#/$defs/{definition}"));
    let validator = jsonschema::options()
        .should_validate_formats(true)
        .build(&schema)
        .unwrap();
    let example = serde_json::to_value(example).unwrap();
    let errors: Vec<String> = validator
        .iter_errors(&example)
        .map(|e| format!("{}: {e}", e.instance_path()))
        .collect();
    assert!(errors.is_empty(), "{definition} {example}: {errors:?}");
}

fn text() -> I18nText {
    I18nText::new("Ménage", "Cleaning").with("de", "Reinigung")
}

#[test]
fn stats_summary_examples_fit_the_schema() {
    let args = StatsSummaryArgs {
        property_id: Uuid::new_v4(),
        period: 90,
        key: "reports".into(),
    };
    assert_valid(STATS_SUMMARY_SCHEMA_V1, "StatsSummaryArgs", args);
    assert_valid(
        STATS_SUMMARY_SCHEMA_V1,
        "StatsSummary",
        stats::summary("0", text()),
    );
    assert_valid(
        STATS_SUMMARY_SCHEMA_V1,
        "StatsSummary",
        stats::summary("123456789012", text())
            .attention(AttentionLevel::Warning, text())
            .trend("-10 %", TrendDirection::Down, true),
    );
}

#[test]
fn timeline_examples_fit_the_schema() {
    let property_id = Uuid::new_v4();
    let stay_id = Uuid::new_v4();
    let at = Utc.with_ymd_and_hms(2026, 9, 26, 10, 0, 0).unwrap();

    let args = TimelineTasksArgs {
        property_id,
        from: at,
        to: at,
        stays: vec![TimelineStay {
            id: stay_id,
            check_in: at,
            check_out: at,
            guest_name: "Liam".into(),
            status: "UPCOMING".into(),
        }],
    };
    assert_valid(TIMELINE_TASKS_SCHEMA_V1, "TimelineTasksArgs", args);

    let task = timeline::task(
        format!("cleaning:{stay_id}"),
        at,
        property_id,
        text(),
        text(),
    )
    .stay(stay_id)
    .due_at(at)
    .assignee("Julie Martin", text())
    .items(vec![
        TimelineTaskItem::new("floors", text()),
        TimelineTaskItem {
            done: true,
            photo: Some(format!("portaki-file:{}", Uuid::new_v4())),
            ..TimelineTaskItem::new("living-room", text()).photo_required()
        },
    ]);
    assert_valid(
        TIMELINE_TASKS_SCHEMA_V1,
        "TimelineTasks",
        TimelineTasks { tasks: vec![task] },
    );

    let toggle = TaskToggleArgs {
        property_id,
        task_id: "cleaning:x".into(),
        item_id: "floors".into(),
        done: true,
        photo: None,
    };
    assert_valid(TIMELINE_TASKS_SCHEMA_V1, "TaskToggleArgs", toggle);
    let complete = TaskCompleteArgs {
        property_id,
        task_id: "cleaning:x".into(),
    };
    assert_valid(TIMELINE_TASKS_SCHEMA_V1, "TaskCompleteArgs", complete);
    let updated = TaskUpdated {
        property_id,
        stay_id: Some(stay_id),
        task_id: "cleaning:x".into(),
        done: 3,
        total: 7,
        assignee_name: Some("Julie Martin".into()),
    };
    assert_valid(TIMELINE_TASKS_SCHEMA_V1, "TaskUpdated", updated);
}

#[test]
fn publish_readiness_example_fits_the_schema() {
    let readiness = PublishReadiness {
        items: vec![PublishCheck {
            id: "entry-code".into(),
            level: PublishLevel::Required,
            ok: false,
            label: text(),
            hint: text(),
        }],
    };
    assert_valid(PUBLISH_READINESS_SCHEMA_V1, "PublishReadiness", readiness);
}

/// The schemas are strict where the Rust types cannot be: a missing language, a long value.
#[test]
fn the_schemas_refuse_what_the_contract_forbids() {
    let mut schema: Value = serde_json::from_str(STATS_SUMMARY_SCHEMA_V1).unwrap();
    schema["$ref"] = Value::String("#/$defs/StatsSummary".into());
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert!(!validator.is_valid(&serde_json::json!({ "value": "1", "label": { "fr": "x" } })));
    assert!(!validator.is_valid(&serde_json::json!({
        "value": "1234567890123", "label": { "fr": "x", "en": "x" }
    })));
}
