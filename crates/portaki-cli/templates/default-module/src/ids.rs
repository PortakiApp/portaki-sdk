//! Typed surface / operation catalogs for this module.

use portaki_sdk::prelude::*;

define_surface_ids! {
    HOME_CARD = "home.card",
    HOST_MAIN = "main",
}

// Declaration sites — `#[query]`, `#[command]` — need the literal at expand time; use sites
// take these constants, so a renamed operation breaks at compile time rather than at runtime.
define_operation_names! {
    GET_CONFIG = "getConfig",
    UPDATE_CONFIG = "updateConfig",
}
