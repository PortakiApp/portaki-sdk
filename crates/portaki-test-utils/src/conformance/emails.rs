//! Declared emails compose around a mock stay.

use chrono::Duration;
use portaki_sdk::email::{EmailContextArgs, EmailTemplateKey};
use portaki_sdk::wasm::registry::{HandlerDeclaration, HandlerKind};
use serde_json::Value;

use super::invoke::{describe, invoke, with_module_id, Invocation, Outcome};
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
                locale: None,
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
    problems
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

fn find(
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
            let mock = MockContextBuilder::guest()
                .with_stay(booking.clone())
                .with_guest_contact(Some("guest@example.com"), Some("+33600000000"))
                .with_now(now);
            let mut mock = with_module_id(mock, module_id);
            mock.context.surface = None;
            (moment, mock)
        })
        .collect()
}
