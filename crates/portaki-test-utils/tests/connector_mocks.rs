//! The built-in connector mocks, as a module test sees them.

use portaki_connectors::open_weather::{CurrentArgs, ForecastArgs, OpenWeather};
use portaki_connectors::osm_nominatim::OsmNominatim;
use portaki_test_utils::MockContext;

#[test]
fn one_line_mounts_every_builtin_connector() {
    MockContext::guest().with_builtin_connectors().run(|_ctx| {
        let now = OpenWeather::current(&CurrentArgs {
            lat: 43.55,
            lng: 7.01,
        })
        .expect("current");
        assert_eq!(now.temp_c, 21.5);
        assert_eq!(now.city_name.as_deref(), Some("MOCK Cannes"));

        let place = OsmNominatim::geocode("anything").expect("geocode");
        assert!(place.display_name.starts_with("MOCK "));
    });
}

#[test]
fn the_same_call_answers_the_same_thing_every_time() {
    let read = || {
        MockContext::guest().with_builtin_connectors().run(|_ctx| {
            let forecast = OpenWeather::forecast(&ForecastArgs {
                lat: 43.55,
                lng: 7.01,
                days: 5,
            })
            .expect("forecast");
            serde_json::to_string(&forecast).expect("serialize")
        })
    };
    assert_eq!(read(), read());
}

#[test]
fn an_explicit_stub_wins_over_the_builtin_one() {
    // Whichever order the builder calls come in.
    for builtin_first in [true, false] {
        let mut builder = MockContext::guest();
        if builtin_first {
            builder = builder.with_builtin_connectors();
        }
        builder = builder.with_connector_response(
            "open-weather",
            "current",
            r#"{"portakiMock":true,"name":"MOCK Antibes","main":{"temp":-4.0,"humidity":90}}"#,
        );
        if !builtin_first {
            builder = builder.with_builtin_connectors();
        }
        builder.run(|_ctx| {
            let now = OpenWeather::current(&CurrentArgs {
                lat: 43.55,
                lng: 7.01,
            })
            .expect("current");
            assert_eq!(now.temp_c, -4.0, "builtin_first={builtin_first}");
        });
    }
}

#[test]
fn a_custom_connector_brings_its_own_table() {
    const ACME_PMS_MOCKS: &[(&str, &str)] = &[(
        "reservations",
        r#"{"portakiMock":true,"rows":[{"id":"mock-res-1","guest":"MOCK Dupont"}]}"#,
    )];

    MockContext::host()
        .with_connector_mocks("acme-pms", ACME_PMS_MOCKS)
        .run(|_ctx| {
            let rows: serde_json::Value =
                portaki_sdk::host::connectors::call("acme-pms", "reservations", &()).expect("call");
            assert_eq!(rows["rows"][0]["guest"], "MOCK Dupont");
        });
}
