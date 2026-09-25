//! Declared emails compose around a mock stay.

use chrono::Duration;
use portaki_sdk::email::{EmailContextArgs, EmailTemplateKey};
use portaki_sdk::wasm::registry::{HandlerDeclaration, HandlerKind};
use serde_json::Value;

use super::invoke::{describe, invoke, invoke_dispatch, with_module_id, Invocation, Outcome};
use super::surfaces::module_id;
use super::{Findings, Module, MANIFEST_FILE};
use crate::{Booking, MockContextBuilder};

/// The query the platform asks for email snippets, by convention.
pub const EMAIL_CONTEXT_QUERY: &str = "emailContext";

pub(super) fn check(module: &Module) -> Result<(), Findings> {
    Findings::of("emails", problems(module))
}

/// Every email command and every `emailContext` composition, around a stay seen before check-in
/// and after check-out.
pub(super) fn compose_all(module: &Module) -> (Vec<String>, Vec<(String, Invocation)>) {
    let declarations = module.declarations();
    let module_id = module_id(module);
    let booking = Booking::default();
    let mut problems = Vec::new();
    let mut invocations = Vec::new();

    for (email_id, command) in declared_email_commands(module) {
        let Some(declaration) = find(&declarations, HandlerKind::Command, &command) else {
            problems.push(format!(
                "{MANIFEST_FILE} email `{email_id}` dispatches command `{command}`, but no \
                 #[command(name = \"{command}\")] is declared"
            ));
            continue;
        };
        for (moment, mock) in stay_mocks(&booking, module_id.as_deref()) {
            let label = format!("email `{email_id}` → {} {moment}", describe(declaration));
            let invocation = invoke(declaration, mock, Value::Object(serde_json::Map::new()));
            invocations.push((label, invocation));
        }
    }

    if let Some(declaration) = find(&declarations, HandlerKind::Query, EMAIL_CONTEXT_QUERY) {
        let keys = std::iter::once(None).chain(EmailTemplateKey::ALL.iter().copied().map(Some));
        for key in keys {
            let args = EmailContextArgs {
                template_key: key,
                stay_id: Some(booking.id.to_string()),
                ..Default::default()
            };
            let params = serde_json::to_value(&args).expect("EmailContextArgs serializes");
            let template = key
                .and_then(|key| serde_json::to_value(key).ok())
                .and_then(|value| value.as_str().map(str::to_string))
                .unwrap_or_else(|| "no template".to_string());
            for (moment, mock) in stay_mocks(&booking, module_id.as_deref()) {
                let label = format!("{} for `{template}` {moment}", describe(declaration));
                invocations.push((label, invoke(declaration, mock, params.clone())));
            }
        }
    }

    (problems, invocations)
}

fn problems(module: &Module) -> Vec<String> {
    let (mut problems, invocations) = compose_all(module);
    problems.extend(invocations.into_iter().filter_map(|(label, invocation)| {
        match invocation.outcome {
            Outcome::Panicked(message) => Some(format!("{label} panicked: {message}")),
            Outcome::Answered(_) | Outcome::Failed(_) => None,
        }
    }));
    problems.extend(declared_vars_problems(module));
    problems
}

/// Every variable `#[email_vars]` declares comes back, non-blank, on the module's fixture — and
/// `emailContext` is only the generated one.
fn declared_vars_problems(module: &Module) -> Vec<String> {
    let mut problems = Vec::new();
    let email_contexts = module
        .declarations()
        .iter()
        .filter(|d| d.kind == HandlerKind::Query && d.name == EMAIL_CONTEXT_QUERY)
        .count();
    if email_contexts > 1 {
        problems.push(format!(
            "{email_contexts} queries are named `{EMAIL_CONTEXT_QUERY}` — #[email_vars] generates \
             it: remove the hand-written one"
        ));
    }
    let module_id = module_id(module);
    let booking = Booking::default();
    for declaration in portaki_sdk::email::declarations() {
        let Some(fixture) = module.email_fixture() else {
            problems.push(
                "#[email_vars] declares variables, and the battery has no data to ask them of — \
                 give it a fixture: conformance!(email_fixture = my_fixture), \
                 fn my_fixture(EmailTemplateKey, MockContextBuilder) -> MockContextBuilder"
                    .to_string(),
            );
            break;
        };
        for (template, vars) in declaration.declared {
            let args = EmailContextArgs {
                template_key: Some(*template),
                stay_id: Some(booking.id.to_string()),
                checkin_time_formatted: Some("16:00".into()),
                address_hint: Some("Antibes".into()),
                ..Default::default()
            };
            let mock = fixture(
                *template,
                stay_mock_for(*template, &booking, module_id.as_deref()),
            );
            let params = serde_json::to_value(&args).expect("EmailContextArgs serializes");
            let label = format!("#[email_vars] for `{template}` on the fixture");
            match invoke_dispatch(declaration.dispatch, mock, params).outcome {
                Outcome::Answered(answer) => {
                    for var in *vars {
                        let given = answer
                            .get(var.as_str())
                            .and_then(Value::as_str)
                            .is_some_and(|text| !text.trim().is_empty());
                        if !given {
                            problems.push(format!(
                                "{label} declares `{var}` and does not return it: {answer}"
                            ));
                        }
                    }
                }
                Outcome::Failed(error) => problems.push(format!("{label} failed: {error}")),
                Outcome::Panicked(message) => problems.push(format!("{label} panicked: {message}")),
            }
        }
    }
    problems
}

/// The stay as the template sees it: the day before check-in, check-in day, mid-stay, or two
/// days after check-out.
fn stay_mock_for(
    template: EmailTemplateKey,
    booking: &Booking,
    module_id: Option<&str>,
) -> MockContextBuilder {
    let now = match template {
        EmailTemplateKey::ArrivalDay => booking.check_in,
        EmailTemplateKey::PostArrival => booking.check_in + Duration::days(1),
        EmailTemplateKey::LostFound => booking.check_out + Duration::days(2),
        _ => booking.check_in - Duration::days(1),
    };
    let mut mock = with_module_id(guest_stay_mock(booking, now), module_id);
    mock.context.surface = None;
    mock
}

fn guest_stay_mock(booking: &Booking, now: chrono::DateTime<chrono::Utc>) -> MockContextBuilder {
    MockContextBuilder::guest()
        .with_stay(booking.clone())
        .with_guest_contact(Some("guest@example.com"), Some("+33600000000"))
        .with_now(now)
}

/// `(email id, command)` for every `emails[]` entry that names a command — top level or in its
/// trigger. An email without one is composed by the platform, not the module.
fn declared_email_commands(module: &Module) -> Vec<(String, String)> {
    let Ok(Some(manifest)) = module.manifest() else {
        return Vec::new();
    };
    manifest
        .get("emails")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|email| {
            let id = email.get("id").and_then(Value::as_str).unwrap_or("?");
            let command = email
                .get("command")
                .or_else(|| email.get("trigger")?.get("command"))
                .and_then(Value::as_str)?;
            Some((id.to_string(), command.to_string()))
        })
        .collect()
}

pub(super) fn find(
    declarations: &[&'static HandlerDeclaration],
    kind: HandlerKind,
    name: &str,
) -> Option<&'static HandlerDeclaration> {
    declarations
        .iter()
        .copied()
        .find(|declaration| declaration.kind == kind && declaration.name == name)
}

/// A guest mock around the default booking, the clock before its check-in then after its
/// check-out — when timed emails go out.
fn stay_mocks(
    booking: &Booking,
    module_id: Option<&str>,
) -> Vec<(&'static str, MockContextBuilder)> {
    let before = booking.check_in - Duration::days(1);
    let after = booking.check_out + Duration::days(2);
    [("before check-in", before), ("after check-out", after)]
        .into_iter()
        .map(|(moment, now)| {
            let mut mock = with_module_id(guest_stay_mock(booking, now), module_id);
            mock.context.surface = None;
            (moment, mock)
        })
        .collect()
}
