//! Canned connector responses for module tests — **never part of a published module**.
//!
//! # Where it lives
//!
//! Each connector carries its own table next to the code that parses it
//! (`open_weather::MOCK_RESPONSES`, …), because only the connector knows the shape
//! `host::connectors::call` deserializes. [`ALL`] aggregates them; adding a connector
//! is one `const` plus one line here.
//!
//! # Boundary
//!
//! The whole module is behind the `mock` Cargo feature, **off by default**. The only
//! crate that turns it on is `portaki-test-utils`, which a module declares as a
//! `[dev-dependencies]` — and with resolver v2 a dev-dependency's features are not
//! unified into a build that does not compile test targets. `cargo build --release
//! --target wasm32-unknown-unknown` therefore never compiles a byte of this module.
//! `scripts/check-mock-boundary.sh` proves it on the produced artifact.
//!
//! # Convention: plausible, never credible
//!
//! Every payload is a JSON object carrying [`MARKER_FIELD`] set to `true`, every
//! human-readable string starts with `MOCK ` and every identifier with `mock-`.
//! Numbers look like the real thing so a screen lays out correctly; the text on that
//! screen says out loud that it is simulated.
//!
//! # Determinism
//!
//! The tables are `const`. Same call, same bytes, forever — including dates, which are
//! fixed calendar days rather than offsets from the clock.
//!
//! # Use
//!
//! Module tests do not read these tables directly — one line on the mock host registers
//! them all:
//!
//! ```ignore
//! portaki_test_utils::MockContext::guest()
//!     .with_builtin_connectors()
//!     .run(|_ctx| { /* OpenWeather::current(…) answers from the table above */ });
//! ```

/// Root field every mock payload carries, set to `true`.
///
/// A surface that renders a mock answer can look for it; a test asserts it is there.
pub const MARKER_FIELD: &str = "portakiMock";

/// Every built-in connector's mock table: `(connector_id, [(operation, response_json)])`.
///
/// Matches the ids of [`portaki_sdk::host::connectors::call`].
pub const ALL: &[(&str, &[(&str, &str)])] = &[
    ("nuki", crate::nuki::MOCK_RESPONSES),
    ("open-weather", crate::open_weather::MOCK_RESPONSES),
    ("osm-nominatim", crate::osm_nominatim::MOCK_RESPONSES),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_mock_is_a_marked_json_object() {
        for (connector_id, operations) in ALL {
            assert!(!operations.is_empty(), "{connector_id} has no mock");
            for (operation, json) in *operations {
                let value: serde_json::Value = serde_json::from_str(json)
                    .unwrap_or_else(|err| panic!("{connector_id}/{operation}: {err}"));
                assert_eq!(
                    value.get(MARKER_FIELD).and_then(serde_json::Value::as_bool),
                    Some(true),
                    "{connector_id}/{operation} must carry \"{MARKER_FIELD}\": true"
                );
            }
        }
    }

    #[test]
    fn every_mock_name_says_it_is_simulated() {
        // A phrase — several words, at least one letter — is what reaches a screen and
        // must announce itself. Bare codes ("unlock", "Clear") and timestamps are not
        // names and cannot be read as provider data on their own.
        fn is_phrase(text: &str) -> bool {
            text.contains(' ') && text.contains(|c: char| c.is_ascii_alphabetic())
        }
        fn walk(value: &serde_json::Value, where_: &str) {
            match value {
                serde_json::Value::String(text) if is_phrase(text) => assert!(
                    text.starts_with("MOCK") || text.starts_with("mock-"),
                    "{where_}: {text:?} must start with MOCK / mock-"
                ),
                serde_json::Value::Array(items) => {
                    items.iter().for_each(|item| walk(item, where_));
                }
                serde_json::Value::Object(fields) => {
                    fields.values().for_each(|field| walk(field, where_));
                }
                _ => {}
            }
        }
        for (connector_id, operations) in ALL {
            for (operation, json) in *operations {
                walk(
                    &serde_json::from_str(json).expect("valid JSON"),
                    &format!("{connector_id}/{operation}"),
                );
            }
        }
    }
}
