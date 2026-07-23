//! Temperature primitive typed fields (SDK max typing).

use portaki_sdk::sdui::common::{TempVariant, TemperatureUnit};
use portaki_sdk::sdui::primitives::Temperature;

#[test]
fn temperature_serializes_variant_snake_case() {
    let node = Temperature::new()
        .value(21.0)
        .unit(TemperatureUnit::Celsius)
        .variant(TempVariant::Hero);

    let json = serde_json::to_value(&node).unwrap();
    assert_eq!(json["variant"], "hero");
    assert_eq!(json["unit"], "C");
    assert_eq!(json["value"], 21.0);
}

#[test]
fn temperature_variant_defaults_to_none() {
    let node = Temperature::new().value(10.0);
    let json = serde_json::to_value(&node).unwrap();
    assert!(json.get("variant").is_none());
}
