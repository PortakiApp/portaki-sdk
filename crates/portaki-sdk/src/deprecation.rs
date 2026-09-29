//! What the platform is withdrawing, since when, and what to use instead.
//!
//! # Why here rather than in a file
//!
//! A capability is declared in Rust; its deprecation is declared in the same place, otherwise
//! the two drift apart the day one moves without the other. The JSON document the registry
//! distributes is **produced** from this table, and a test checks that the checked-in one has
//! not drifted — that is what makes the duplication safe.
//!
//! # How it travels
//!
//! The SDK's CI publishes `deprecations.json` along with the other contracts of a release, on
//! `/registry/v1/sdk-releases`. `portaki ci check` reads it back and warns a module that still
//! relies on something on its way out. Nothing ever fails because of this: a deprecation warns,
//! it does not forbid.
//!
//! # Examples
//!
//! ```
//! use portaki_sdk::deprecation;
//!
//! // Nothing is deprecated today; the mechanism itself still answers.
//! assert!(deprecation::find("core.storage").is_none());
//! ```

use serde::{Deserialize, Serialize};

/// What a deprecated identifier refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Subject {
    /// A capability from [`crate::capability`].
    Capability,
    /// A connector declared by a module.
    Connector,
    /// A host operation from `host-ops.json`.
    HostOp,
}

/// An announced withdrawal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Deprecation {
    /// The identifier on its way out — `core.storage`, `nuki`, `kv.list`.
    pub id: &'static str,
    /// What it is.
    pub subject: Subject,
    /// The SDK version from which it is deprecated.
    pub since: &'static str,
    /// What to replace it with, when a replacement exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replacement: Option<&'static str>,
    /// What a module author needs to know in order to act.
    pub note: &'static str,
}

/// Everything deprecated to date.
///
/// Empty, and rightly so: nothing has been withdrawn yet. The table exists so that the first
/// deprecation is a line to add, not a mechanism to design in a hurry — at the exact moment
/// when what you want is to warn the authors, not to build.
pub const DEPRECATIONS: &[Deprecation] = &[];

/// What is deprecated under this identifier, if anything is.
pub fn find(id: &str) -> Option<&'static Deprecation> {
    DEPRECATIONS.iter().find(|entry| entry.id == id)
}

/// The document the CI publishes to the registry, in the form it must be checked in.
///
/// A function rather than a reference file: the content comes from [`DEPRECATIONS`], and the
/// JSON in the repository is only a fingerprint of it, verified by the tests.
pub fn contract() -> serde_json::Value {
    serde_json::json!({
        "description":
            "Capabilities, connectors and host ops being withdrawn — advisory, never blocking",
        "deprecations": DEPRECATIONS,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The checked-in JSON is a fingerprint of the table, not a second source.
    ///
    /// Without this test, adding a deprecation in Rust without regenerating the file would
    /// publish a contract that does not describe the SDK — the very mistake the table is
    /// meant to prevent.
    #[test]
    fn the_checked_in_contract_matches_the_table() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../contracts/deprecations.json");
        let Ok(raw) = std::fs::read_to_string(&path) else {
            // The packaged crate does not carry the repository's folder along; there is then
            // nothing to compare, and nothing to report.
            return;
        };
        let versioned: serde_json::Value =
            serde_json::from_str(&raw).expect("contracts/deprecations.json est du JSON");

        assert_eq!(
            versioned,
            contract(),
            "contracts/deprecations.json a dérivé de deprecation::DEPRECATIONS"
        );
    }

    #[test]
    fn an_unknown_id_is_not_deprecated() {
        assert!(find("core.storage").is_none());
        assert!(find("nothing.at.all").is_none());
    }

    /// The document's shape matters as much as its content: the shape is what the CLI reads.
    #[test]
    fn a_deprecation_serialises_as_the_cli_reads_it() {
        let entry = Deprecation {
            id: "core.storage",
            subject: Subject::Capability,
            since: "2.4.0",
            replacement: Some("core.kv"),
            note: "typed repositories replace raw storage",
        };

        let rendered = serde_json::to_value(&entry).unwrap();

        assert_eq!(rendered["id"], "core.storage");
        assert_eq!(rendered["subject"], "capability");
        assert_eq!(rendered["since"], "2.4.0");
        assert_eq!(rendered["replacement"], "core.kv");
    }

    /// With no replacement, the key disappears instead of showing up null: a reader tells
    /// "no replacement" apart from "unknown replacement" with no extra convention.
    #[test]
    fn a_deprecation_without_a_replacement_omits_the_field() {
        let entry = Deprecation {
            id: "kv.list",
            subject: Subject::HostOp,
            since: "2.4.0",
            replacement: None,
            note: "unbounded listing never scaled",
        };

        let rendered = serde_json::to_value(&entry).unwrap();

        assert!(rendered.get("replacement").is_none());
    }
}
