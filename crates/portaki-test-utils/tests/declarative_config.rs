//! A module that declares its configuration with `#[portaki_sdk::config]`.
//!
//! The platform owns the config: it hands it over as `context.moduleConfig`, and until it has
//! imported a config kept in KV, the module reads that — through `legacyConfig` for the platform,
//! through `Config::load` for itself.

use portaki_sdk::host::module::ModuleStatus;
use portaki_sdk::prelude::*;
use portaki_sdk::wasm::registry::{declarations, HandlerKind};
use portaki_test_utils::MockContext;

#[wire]
#[derive(PartialEq, Eq, Default)]
pub struct Contact {
    pub name: String,
    pub phone: String,
}

#[portaki_sdk::config]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    #[field(required, label = "config.ssid")]
    pub ssid: String,
    #[field(secret, recommended, label = "config.password")]
    pub password: String,
    #[field(structured, label = "config.contacts")]
    pub contacts: Vec<Contact>,
    pub hidden: bool,
}

#[portaki_sdk::query(name = "wifi")]
pub fn wifi(ctx: Context) -> Result<Config> {
    Config::load(&ctx)
}

fn villa() -> Config {
    Config {
        ssid: "Villa-Azur".into(),
        password: "s3cret".into(),
        contacts: vec![Contact {
            name: "Julie".into(),
            phone: "+33600000000".into(),
        }],
        hidden: true,
    }
}

#[test]
fn the_module_reads_what_the_platform_hands_over() {
    MockContext::host()
        .with_config(&villa())
        .with_kv("config", br#"{"ssid":"old"}"#.to_vec())
        .run(|ctx| assert_eq!(wifi(ctx).unwrap(), villa()));
}

/// Only the keys the host filled in are stored: the others take their default.
#[test]
fn a_partial_config_fills_in_defaults() {
    MockContext::host()
        .with_config(&serde_json::json!({ "ssid": "Villa-Azur", "password": null }))
        .run(|ctx| {
            let config = wifi(ctx).unwrap();
            assert_eq!(config.ssid, "Villa-Azur");
            assert_eq!(config.password, "");
            assert!(config.contacts.is_empty());
        });
}

#[test]
fn before_the_import_the_kv_config_is_read() {
    MockContext::host()
        .with_kv(
            "config",
            br#"{"ssid":"Mas-Provence","hidden":true}"#.to_vec(),
        )
        .run(|ctx| {
            let config = wifi(ctx).unwrap();
            assert_eq!(config.ssid, "Mas-Provence");
            assert!(config.hidden);
        });
    MockContext::host().run(|ctx| assert_eq!(wifi(ctx).unwrap(), Config::default()));
}

#[test]
fn an_unreadable_config_fails_instead_of_resetting() {
    MockContext::host()
        .with_config(&serde_json::json!({ "contacts": "Julie" }))
        .run(|ctx| {
            let error = wifi(ctx).unwrap_err();
            assert!(error.to_string().contains("config_unreadable"), "{error}");
        });
}

/// The generated host query the platform calls once to import the KV config.
#[test]
fn legacy_config_answers_the_raw_kv_blob() {
    let legacy = declarations()
        .find(|declaration| declaration.name == "legacyConfig")
        .expect("#[config] declares legacyConfig");
    assert_eq!(legacy.kind, HandlerKind::Query);

    MockContext::host()
        .with_kv("config", br#"{"ssid":"old","extra":1}"#.to_vec())
        .run(|ctx| {
            let raw = (legacy.dispatch)(ctx, serde_json::json!({})).unwrap();
            assert_eq!(raw, serde_json::json!({ "ssid": "old", "extra": 1 }));
        });
    MockContext::host().run(|ctx| {
        let raw = (legacy.dispatch)(ctx, serde_json::json!({})).unwrap();
        assert_eq!(raw, serde_json::Value::Null);
    });
}

#[test]
fn the_mock_can_report_an_incomplete_install() {
    MockContext::host()
        .with_module_status(ModuleStatus {
            active: true,
            workspace_enabled: true,
            incomplete: true,
            requires_config: true,
            missing_required_keys: vec!["ssid".into()],
        })
        .run(|_ctx| {
            let status = host::module::status().unwrap();
            assert!(status.incomplete);
            assert!(!status.is_ready());
            assert_eq!(status.missing_required_keys, ["ssid"]);
        });
    MockContext::host().run(|_ctx| assert!(host::module::status().unwrap().is_ready()));
}

portaki_test_utils::conformance!(
    dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/config")
);
