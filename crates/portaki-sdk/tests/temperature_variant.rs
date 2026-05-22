//! Temperature primitive variant field (Phase 4 SDK hardening).

use portaki_sdk::sdui::common::TempVariant;
use portaki_sdk::sdui::primitives::Temperature;

#[test]
fn temperature_serializes_variant_snake_case() {
    let node = Temperature::new()
        .value(serde_json::json!(21))
        .unit(serde_json::json!("celsius"))
        .variant(TempVariant::Hero);

    let json = serde_json::to_value(&node).unwrap();
    assert_eq!(json["variant"], "hero");
}

#[test]
fn temperature_variant_defaults_to_none() {
    let node = Temperature::new().value(serde_json::json!(10));
    let json = serde_json::to_value(&node).unwrap();
    assert!(json.get("variant").is_none());
}
