//! Text the dashboard shows as is, in the viewer's language.
//!
//! Wire: a flat `{ "fr": "…", "en": "…", "de": "…" }` — `fr` and `en` required, other locales
//! beside them. Not [`crate::host::email::LocalizedEmailText`], whose extra locales nest under
//! `translations`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Localized display text: `fr` and `en` always, other locales optional.
///
/// ```
/// use portaki_sdk::contracts::i18n::I18nText;
///
/// let text = I18nText::new("Ménage", "Cleaning").with("de", "Reinigung");
/// assert_eq!(
///     serde_json::to_value(&text).unwrap(),
///     serde_json::json!({ "fr": "Ménage", "en": "Cleaning", "de": "Reinigung" })
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct I18nText {
    /// French.
    pub fr: String,
    /// English.
    pub en: String,
    /// Any other locale, keyed by language tag.
    #[serde(flatten)]
    pub others: BTreeMap<String, String>,
}

impl I18nText {
    /// French and English.
    pub fn new(fr: impl Into<String>, en: impl Into<String>) -> Self {
        Self {
            fr: fr.into(),
            en: en.into(),
            others: BTreeMap::new(),
        }
    }

    /// Adds another locale.
    pub fn with(mut self, locale: impl Into<String>, text: impl Into<String>) -> Self {
        self.others.insert(locale.into(), text.into());
        self
    }
}
