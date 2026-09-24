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
        Ok(None) => {
            return vec![format!(
                "no manifest in {} — run `portaki build`, which writes {BUILT_MANIFEST_FILE} \
                 from the code; the registry has nothing to list the module with until then",
                module.root().display()
            )]
        }
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

    let file = if module.root().join(MANIFEST_FILE).exists() {
        MANIFEST_FILE
    } else {
        BUILT_MANIFEST_FILE
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
