//! `publishReadiness` — what a module still needs before its property goes live.
//!
//! Reserved to conditional rules: an empty config field declared `required` or `recommended`
//! with `#[portaki_sdk::config]` is checked by the platform itself, without this query.
//!
//! The platform asks every active module before publishing a property, with args
//! `{ "propertyId" }` in a host context reading the draft KV. `required` items that are not `ok`
//! block the publication; `recommended` and `optional` never do. A module without this query
//! adds nothing. Schema: `contracts/publish-readiness.v1.json`.
//!
//! ```
//! use portaki_sdk::contracts::i18n::I18nText;
//! use portaki_sdk::contracts::publish::{PublishCheck, PublishLevel, PublishReadiness};
//!
//! let readiness = PublishReadiness {
//!     items: vec![PublishCheck {
//!         id: "entry-code".into(),
//!         level: PublishLevel::Required,
//!         ok: false,
//!         label: I18nText::new("Code d'entrée", "Entry code"),
//!         hint: I18nText::new("Il manque le code d'entrée", "The entry code is missing"),
//!     }],
//! };
//! assert_eq!(serde_json::to_value(&readiness).unwrap()["items"][0]["level"], "required");
//! ```

use serde::{Deserialize, Serialize};

use crate::contracts::i18n::I18nText;
use crate::ids::OperationName;

/// Query name (`publishReadiness`).
pub const PUBLISH_READINESS: OperationName = OperationName::new("publishReadiness");

/// Answer of [`PUBLISH_READINESS`].
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishReadiness {
    /// One per thing the module checks, done or not.
    pub items: Vec<PublishCheck>,
}

/// One point of the pre-publication check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishCheck {
    /// Stable id within the module (`entry-code`).
    pub id: String,
    /// Whether it blocks the publication.
    pub level: PublishLevel,
    /// Done.
    pub ok: bool,
    /// What is checked.
    pub label: I18nText,
    /// What is missing, or why it matters.
    pub hint: I18nText,
}

/// How much a [`PublishCheck`] weighs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PublishLevel {
    /// Blocks the publication while not ok.
    Required,
    /// Shown, never blocks.
    Recommended,
    /// Shown, never blocks.
    Optional,
}
