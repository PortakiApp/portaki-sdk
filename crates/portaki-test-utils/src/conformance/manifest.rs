//! The module's manifest — hand-written or built — against the schema the registry validates with.

use super::{Findings, Module, BUILT_MANIFEST_FILE, MANIFEST_FILE};

/// `schema/module.v1.json` of the SDK, carried in this crate so a check never needs the network.
///
/// A copy, because a published crate cannot reach outside its own directory; a test of this crate
/// fails when it drifts from the repository's `schema/module.v1.json`.
pub const MODULE_SCHEMA_V1: &str = include_str!("../../schema/module.v1.json");

pub(super) fn check(module: &Module) -> Result<(), Findings> {
    Findings::of("manifest", problems(module))
}

fn problems(module: &Module) -> Vec<String> {
    let manifest = match module.manifest() {
        Ok(Some(manifest)) => manifest,
        // Nothing built yet: the manifest is written from the code by `portaki build`, and
        // `portaki lint` validates it right after — `cargo test` alone has nothing to read.
        Ok(None) => return Vec::new(),
        Err(error) => return vec![error],
    };

    let schema: serde_json::Value =
        serde_json::from_str(MODULE_SCHEMA_V1).expect("the bundled module.v1.json is JSON");
    let validator = match jsonschema::validator_for(&schema) {
        Ok(validator) => validator,
        Err(error) => {
            return vec![format!(
                "the bundled module.v1.json does not compile: {error}"
            )]
        }
    };

    let file = if module.root().join(BUILT_MANIFEST_FILE).exists() {
        BUILT_MANIFEST_FILE
    } else {
        MANIFEST_FILE
    };
    validator
        .iter_errors(&manifest)
        .map(|error| {
            let at = error.instance_path().to_string();
            let at = if at.is_empty() { "/".to_string() } else { at };
            format!("{file} at {at}: {error}")
        })
        .collect()
}
