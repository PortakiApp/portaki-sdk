//! The typed answers a host surface commits the module to — stats tile, stats detail, timeline
//! tasks — and `publishReadiness` when it is exported.

use std::time::{Duration, Instant};

use chrono::Duration as Days;
use portaki_sdk::contracts::{publish, stats, timeline};
use portaki_sdk::wasm::registry::{HandlerDeclaration, HandlerKind};
use serde_json::{json, Value};

use super::emails::find;
use super::invoke::{describe, invoke, invoke_in, with_module_id, Outcome};
use super::surfaces::{contract_problems, module_id};
use super::{Findings, Module, MANIFEST_FILE};
use crate::{Booking, MockContextBuilder};

/// `contracts/publish-readiness.v1.json` of the SDK, carried like [`super::MODULE_SCHEMA_V1`].
pub const PUBLISH_READINESS_SCHEMA_V1: &str =
    include_str!("../../schema/publish-readiness.v1.json");
/// `contracts/stats-summary.v1.json` of the SDK.
pub const STATS_SUMMARY_SCHEMA_V1: &str = include_str!("../../schema/stats-summary.v1.json");
/// `contracts/timeline-tasks.v1.json` of the SDK.
pub const TIMELINE_TASKS_SCHEMA_V1: &str = include_str!("../../schema/timeline-tasks.v1.json");

/// Longest a `statsSummary` may take on the fixture: the statistics page waits on every tile.
const STATS_SUMMARY_BUDGET: Duration = Duration::from_millis(300);

const PERIODS: [u32; 3] = [30, 90, 365];

pub(super) fn check(module: &Module) -> Result<(), Findings> {
    Findings::of("contracts", problems(module))
}

fn problems(module: &Module) -> Vec<String> {
    let declarations = module.declarations();
    let module_id = module_id(module);
    let host = || {
        let mut mock = with_module_id(MockContextBuilder::host(), module_id.as_deref());
        mock.context.surface = None;
        mock
    };

    let mut problems = Vec::new();
    for (kind, path) in host_surfaces(module) {
        match kind.as_str() {
            "property-stats-card" => problems.extend(stats_summary(&declarations, host(), &path)),
            "property-stats-detail" => problems.extend(stats_detail(&declarations, host(), &path)),
            "workspace-timeline-task" => problems.extend(timeline_tasks(&declarations, host())),
            _ => {}
        }
    }
    if let Some(query) = find(
        &declarations,
        HandlerKind::Query,
        &publish::PUBLISH_READINESS,
    ) {
        problems.extend(publish_readiness(query, host()));
    }
    problems
}

/// `(type, pathSegment)` of every `hostSurfaces[]` entry.
fn host_surfaces(module: &Module) -> Vec<(String, String)> {
    let Ok(Some(manifest)) = module.manifest() else {
        return Vec::new();
    };
    manifest
        .get("hostSurfaces")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|surface| {
            let text = |key| surface.get(key)?.as_str().map(str::to_string);
            Some((text("type")?, text("pathSegment")?))
        })
        .collect()
}

fn stats_summary(
    declarations: &[&'static HandlerDeclaration],
    mock: MockContextBuilder,
    key: &str,
) -> Vec<String> {
    let Some(query) = find(declarations, HandlerKind::Query, &stats::STATS_SUMMARY) else {
        return vec![format!(
            "{MANIFEST_FILE} declares a property-stats-card `{key}`, but no \
             #[query(name = \"statsSummary\")] is declared"
        )];
    };
    let mut problems = Vec::new();
    for period in PERIODS {
        let args = json!({ "propertyId": mock.context.property_id, "period": period, "key": key });
        let what = format!("{} for key `{key}`, period {period}", describe(query));
        let started = Instant::now();
        let outcome = invoke(query, mock.clone(), args).outcome;
        let elapsed = started.elapsed();
        if let Err(found) = validated(&what, outcome, STATS_SUMMARY_SCHEMA_V1, "StatsSummary") {
            problems.extend(found);
        }
        if elapsed > STATS_SUMMARY_BUDGET {
            problems.push(format!(
                "{what} took {} ms on the fixture — at most {} ms",
                elapsed.as_millis(),
                STATS_SUMMARY_BUDGET.as_millis()
            ));
        }
    }
    problems
}

fn stats_detail(
    declarations: &[&'static HandlerDeclaration],
    mut mock: MockContextBuilder,
    id: &str,
) -> Vec<String> {
    let Some(surface) = declarations
        .iter()
        .copied()
        .find(|d| d.kind == HandlerKind::Surface && d.context == "host" && d.name == id)
    else {
        return vec![format!(
            "{MANIFEST_FILE} declares a property-stats-detail `{id}`, but no \
             #[surface(host, id = \"{id}\")] is declared"
        )];
    };
    mock.context.surface = Some(id.to_string());
    mock.context.input = json!({ "periodDays": 30 });
    let what = format!("{} as property-stats-detail", describe(surface));
    match invoke(surface, mock, Value::Object(Default::default())).outcome {
        Outcome::Answered(tree) => contract_problems(&tree)
            .into_iter()
            .map(|problem| format!("{what} {problem}"))
            .collect(),
        Outcome::Failed(error) => vec![format!("{what} failed on the fixture: {error}")],
        Outcome::Panicked(message) => vec![format!("{what} panicked on the fixture: {message}")],
    }
}

fn timeline_tasks(
    declarations: &[&'static HandlerDeclaration],
    mock: MockContextBuilder,
) -> Vec<String> {
    let Some(query) = find(declarations, HandlerKind::Query, &timeline::TIMELINE_TASKS) else {
        return vec![format!(
            "{MANIFEST_FILE} declares a workspace-timeline-task surface, but no \
             #[query(name = \"timelineTasks\")] is declared"
        )];
    };

    // Three stays back to back around the default booking: before, during and after the window.
    let booking = Booking::default();
    let week = Days::days(7);
    let stays: Vec<Value> = [-1, 0, 1]
        .into_iter()
        .map(|shift| {
            json!({
                "id": uuid::Uuid::new_v4(),
                "checkIn": booking.check_in + week * shift,
                "checkOut": booking.check_out + week * shift,
                "guestName": "Liam Moreau",
                "status": "UPCOMING",
            })
        })
        .collect();
    let (ctx, host) = mock.build();
    let args = json!({
        "propertyId": ctx.property_id,
        "from": booking.check_in - Days::days(1),
        "to": booking.check_out + Days::days(1),
        "stays": stays,
    });

    let what = describe(query);
    let outcome = invoke_in(query, ctx.clone(), host.clone(), args).outcome;
    let answer = match validated(&what, outcome, TIMELINE_TASKS_SCHEMA_V1, "TimelineTasks") {
        Ok(answer) => answer,
        Err(problems) => return problems,
    };
    let answer: timeline::TimelineTasks = match serde_json::from_value(answer) {
        Ok(answer) => answer,
        Err(error) => return vec![format!("{what} answered tasks that do not parse: {error}")],
    };
    let mut problems = Vec::new();

    // Ticking an item that requires a photo, without one, must be refused.
    for task in &answer.tasks {
        for item in task.items.iter().filter(|item| item.photo_required) {
            let Some(toggle) = find(declarations, HandlerKind::Command, &timeline::TASK_TOGGLE)
            else {
                problems.push(format!(
                    "{what} returns items that require a photo, but no \
                     #[command(name = \"taskToggle\")] is declared"
                ));
                return problems;
            };
            let args = json!({
                "propertyId": ctx.property_id,
                "taskId": task.id,
                "itemId": item.id,
                "done": true,
            });
            let what = format!(
                "{} on photoRequired item `{}` of task `{}` without a photo",
                describe(toggle),
                item.id,
                task.id
            );
            match invoke_in(toggle, ctx.clone(), host.clone(), args).outcome {
                Outcome::Failed(error) if error.contains(timeline::PHOTO_REQUIRED) => {}
                Outcome::Failed(error) => problems.push(format!(
                    "{what} was refused without the `{}` code: {error}",
                    timeline::PHOTO_REQUIRED
                )),
                Outcome::Answered(_) => problems.push(format!(
                    "{what} was accepted — refuse it with `{}` (TimelineTaskItem::check_toggle)",
                    timeline::PHOTO_REQUIRED
                )),
                Outcome::Panicked(message) => problems.push(format!("{what} panicked: {message}")),
            }
        }
    }
    problems
}

fn publish_readiness(query: &HandlerDeclaration, mock: MockContextBuilder) -> Vec<String> {
    let args = json!({ "propertyId": mock.context.property_id });
    let outcome = invoke(query, mock, args).outcome;
    validated(
        &describe(query),
        outcome,
        PUBLISH_READINESS_SCHEMA_V1,
        "PublishReadiness",
    )
    .err()
    .unwrap_or_default()
}

/// The answer, when there is one and it fits the `definition` of `schema`.
///
/// An error counts as a problem: the platform reads it as « nothing to show », and a first
/// install is exactly what these queries must answer for.
fn validated(
    what: &str,
    outcome: Outcome,
    schema: &str,
    definition: &str,
) -> Result<Value, Vec<String>> {
    let answer = match outcome {
        Outcome::Answered(answer) => answer,
        Outcome::Failed(error) => {
            return Err(vec![format!("{what} failed on the fixture: {error}")])
        }
        Outcome::Panicked(message) => {
            return Err(vec![format!("{what} panicked on the fixture: {message}")])
        }
    };
    let mut schema: Value = serde_json::from_str(schema).expect("a bundled contract is JSON");
    schema["$ref"] = Value::String(format!("#/$defs/{definition}"));
    let validator = jsonschema::options()
        .should_validate_formats(true)
        .build(&schema)
        .expect("a bundled contract compiles");
    let problems: Vec<String> = validator
        .iter_errors(&answer)
        .map(|error| {
            let at = error.instance_path().to_string();
            let at = if at.is_empty() { "/".to_string() } else { at };
            format!("{what} answered off the {definition} contract at {at}: {error}")
        })
        .collect();
    if problems.is_empty() {
        Ok(answer)
    } else {
        Err(problems)
    }
}
