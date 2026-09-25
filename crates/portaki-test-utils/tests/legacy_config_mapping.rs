//! A module whose old KV config has another shape than its declared keys.
//!
//! `#[portaki_sdk::config(legacy = …)]` maps the blob, for the platform's one-time import
//! (`legacyConfig`) and for `load` until then. One `#[config]` per test binary, as per module.

use portaki_sdk::prelude::*;
use portaki_test_utils::MockContext;
use serde_json::{json, Value};

fn from_v1(old: Value) -> Value {
    json!({ "ssid": old["wifi"]["name"], "guests": old["guests"].as_str().and_then(|n| n.parse::<u32>().ok()) })
}

#[portaki_sdk::config(legacy = from_v1)]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    #[field(required, label = "config.ssid")]
    pub ssid: String,
    #[field(label = "config.guests")]
    pub guests: u32,
}

const V1: &[u8] = br#"{"wifi":{"name":"Mas"},"guests":"4"}"#;

#[test]
fn load_reads_the_kv_through_the_mapping() {
    MockContext::host()
        .with_kv("config", V1.to_vec())
        .run(|ctx| {
            assert_eq!(
                Config::load(&ctx).unwrap(),
                Config {
                    ssid: "Mas".into(),
                    guests: 4
                }
            );
        });
}

#[test]
fn legacy_config_answers_the_mapped_blob() {
    MockContext::host()
        .with_kv("config", V1.to_vec())
        .run(|ctx| {
            let answered = portaki_legacy_config(ctx).unwrap();
            assert_eq!(answered, json!({ "ssid": "Mas", "guests": 4 }));
        });
    MockContext::host().run(|ctx| assert_eq!(portaki_legacy_config(ctx).unwrap(), Value::Null));
}

#[test]
fn module_config_is_never_mapped() {
    MockContext::host()
        .with_config(&json!({ "ssid": "Villa" }))
        .with_kv("config", V1.to_vec())
        .run(|ctx| assert_eq!(Config::load(&ctx).unwrap().ssid, "Villa"));
}
