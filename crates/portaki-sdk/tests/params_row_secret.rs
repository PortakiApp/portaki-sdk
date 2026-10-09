//! A config row's `#[field(secret)]` reaches the shape the platform reads (`item.secret`).
//!
//! The macro used to strip the attribute before reading it: every row secret came out unflagged,
//! and the platform stored it in clear. The unit test read the shape first and never saw it.

use serde::{Deserialize, Serialize};

#[portaki_sdk::params]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Network {
    pub id: String,
    pub ssid: String,
    #[field(secret)]
    pub password: String,
}

#[test]
fn a_row_secret_is_flagged_in_the_expanded_shape() {
    let shape = portaki_sdk::wasm::registry::params_shape("Network").expect("Network is declared");
    let fields = shape["fields"].as_array().expect("fields");
    let flagged: Vec<&str> = fields
        .iter()
        .filter(|field| field["secret"] == true)
        .map(|field| field["name"].as_str().unwrap())
        .collect();
    assert_eq!(flagged, ["password"]);
    // And the attribute is gone from the struct: it still compiles as plain serde.
    let _ = Network::default();
}
