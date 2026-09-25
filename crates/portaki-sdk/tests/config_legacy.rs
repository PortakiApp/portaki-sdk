//! `#[config(legacy = f)]` takes an infallible mapping or one returning a `Result`.

use portaki_sdk::context::Context;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

fn plain(old: Value) -> Value {
    json!({ "ssid": old["name"] })
}

fn fallible(old: Value) -> Result<Value, String> {
    let name = old["name"].as_str().ok_or("no name")?;
    Ok(json!({ "ssid": name }))
}

// One config per module: each generates the same `legacyConfig` handlers.
mod a {
    use super::*;

    #[portaki_sdk::config(legacy = plain)]
    #[derive(Debug, Default, Serialize, Deserialize)]
    pub struct Plain {
        #[field(label = "config.ssid")]
        pub ssid: String,
    }
}

mod b {
    use super::*;

    #[portaki_sdk::config(legacy = fallible, legacy_keys = ["texts/fr"])]
    #[derive(Debug, Default, Serialize, Deserialize)]
    pub struct Fallible {
        #[field(label = "config.ssid")]
        pub ssid: String,
    }
}

#[test]
fn both_mappings_compile_and_load_reads_module_config() {
    let ctx = Context {
        module_config: Some(json!({ "ssid": "A" })),
        ..Context::default()
    };
    assert_eq!(a::Plain::load(&ctx).unwrap().ssid, "A");
    assert_eq!(b::Fallible::load(&ctx).unwrap().ssid, "A");
}
