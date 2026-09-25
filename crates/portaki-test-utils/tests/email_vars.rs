//! A module that declares its email variables with `#[portaki_sdk::email_vars]`.
//!
//! The function computes values; the generated `emailContext` asks it only for declared templates,
//! refuses a variable it did not declare, and the conformance battery checks every declared
//! variable comes back on the module's fixture.

mod common;

use common::{assert_reports, failing};
use portaki_sdk::email::{EmailContextArgs, EmailTemplateKey, EmailVar, EmailVars};
use portaki_sdk::prelude::*;
use portaki_sdk::wasm::registry::{declarations, HandlerKind};
use portaki_test_utils::conformance::Module;
use portaki_test_utils::{MockContext, MockContextBuilder};
use serde_json::json;

#[portaki_sdk::config]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    #[field(label = "config.ssid")]
    pub ssid: String,
    #[field(label = "config.phone")]
    pub phone: String,
    /// A test switch: answer a variable the module did not declare.
    pub stray: bool,
}

#[portaki_sdk::email_vars(
    StayLink | EmailTemplateKey::ArrivalDay => [EmailVar::WifiName],
    Arrival => [WifiName, HostPhone],
)]
pub fn email_vars(ctx: Context, _args: EmailContextArgs) -> Result<EmailVars> {
    let config = Config::load(&ctx)?;
    let mut vars = EmailVars::new()
        .with(EmailVar::WifiName, config.ssid)
        .with(EmailVar::HostPhone, config.phone);
    if config.stray {
        vars.insert(EmailVar::WeatherSummary, "Sunny");
    }
    Ok(vars)
}

fn seeded(_template: EmailTemplateKey, mock: MockContextBuilder) -> MockContextBuilder {
    mock.with_config(&Config {
        ssid: "Villa-Azur".into(),
        phone: "+33600000000".into(),
        stray: false,
    })
}

fn empty(_template: EmailTemplateKey, mock: MockContextBuilder) -> MockContextBuilder {
    mock.with_config(&Config::default())
}

fn module() -> Module {
    Module::at(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/email-vars"
    ))
}

fn email_context(config: Config, args: serde_json::Value) -> Result<serde_json::Value> {
    let declaration = declarations()
        .find(|d| d.kind == HandlerKind::Query && d.name == "emailContext")
        .expect("#[email_vars] generates emailContext");
    let (ctx, host) = MockContext::host().with_config(&config).build();
    portaki_sdk::host::with_host(host, ctx.clone(), || (declaration.dispatch)(ctx, args))
}

fn villa() -> Config {
    Config {
        ssid: "Villa-Azur".into(),
        phone: " ".into(),
        stray: false,
    }
}

#[test]
fn the_platform_gets_the_declared_values_of_its_template_and_no_blank_one() {
    let answer = email_context(villa(), json!({ "templateKey": "arrival" })).unwrap();
    assert_eq!(answer, json!({ "wifiName": "Villa-Azur" }));
}

#[test]
fn an_undeclared_template_or_none_is_not_asked() {
    for args in [json!({ "templateKey": "post-arrival" }), json!({})] {
        assert_eq!(email_context(villa(), args).unwrap(), json!({}));
    }
}

#[test]
fn a_variable_is_sent_only_to_the_templates_it_is_declared_for() {
    let mut config = villa();
    config.phone = "+33600000000".into();
    // HostPhone is declared for arrival, not for arrival-day.
    let answer = email_context(config, json!({ "templateKey": "arrival-day" })).unwrap();
    assert_eq!(answer, json!({ "wifiName": "Villa-Azur" }));
}

#[test]
fn a_variable_declared_nowhere_is_an_error() {
    let mut config = villa();
    config.stray = true;
    let error = email_context(config, json!({ "templateKey": "arrival" })).unwrap_err();
    assert!(error.to_string().contains("`weatherSummary`"), "{error}");
}

#[test]
fn the_manifest_declares_what_the_code_does() {
    module()
        .check_manifest()
        .unwrap_or_else(|findings| panic!("{findings}"));
}

#[test]
fn every_declared_variable_comes_back_on_the_fixture() {
    module()
        .with_email_fixture(seeded)
        .check_emails()
        .unwrap_or_else(|findings| panic!("{findings}"));
}

#[test]
fn a_declared_variable_the_fixture_does_not_get_is_reported() {
    let findings = failing("emails", module().with_email_fixture(empty).check_emails());
    assert_reports(&findings, &["`stay-link`", "declares `wifiName`"]);
    assert_reports(&findings, &["`arrival-day`", "declares `wifiName`"]);
}

#[test]
fn without_a_fixture_the_battery_asks_for_one() {
    let findings = failing("emails", module().check_emails());
    assert_reports(&findings, &["#[email_vars]", "email_fixture"]);
}

#[test]
fn the_catalogue_says_where_each_variable_renders() {
    assert!(EmailVar::WifiName.renders_in(EmailTemplateKey::Arrival));
    assert!(!EmailVar::WeatherSummary.renders_in(EmailTemplateKey::Arrival));
    for var in EmailVar::ALL {
        assert!(!var.templates().is_empty(), "{var}");
        assert!(var.templates().iter().all(|t| t.is_guest_stay()), "{var}");
        assert_eq!(serde_json::to_value(var).unwrap(), var.as_str());
    }
}
