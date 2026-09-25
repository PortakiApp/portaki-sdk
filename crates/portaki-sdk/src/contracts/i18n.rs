//! Text in several languages — shown by the dashboard as is, or held in a module's config.
//!
//! Wire: a flat `{ "fr": "…", "en": "…", "de": "…" }`, short language codes. Not
//! [`crate::host::email::LocalizedEmailText`], whose extra locales nest under `translations`.
//!
//! A config field typed [`I18nText`] is a `localized` field of `#[portaki_sdk::config]`: the
//! platform stores one text per language and a save from the host form writes the host's
//! language only, keeping the others.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::context::Context;

/// Localized text: one entry per language code.
///
/// Serializes `fr` and `en` always, other languages beside them. Reads an object — a missing
/// language is empty — or a plain string, the same text in every language (a config saved before
/// it was translated).
///
/// ```
/// use portaki_sdk::contracts::i18n::I18nText;
///
/// let text = I18nText::new("Ménage", "Cleaning").with("de", "Reinigung");
/// assert_eq!(
///     serde_json::to_value(&text).unwrap(),
///     serde_json::json!({ "fr": "Ménage", "en": "Cleaning", "de": "Reinigung" })
/// );
///
/// let legacy: I18nText = serde_json::from_value(serde_json::json!("Bienvenue")).unwrap();
/// assert_eq!(legacy.get("es-ES"), "Bienvenue");
///
/// let partial: I18nText = serde_json::from_value(serde_json::json!({ "en": "Welcome" })).unwrap();
/// assert_eq!(partial.get("fr"), "Welcome"); // fr empty → en
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(from = "Wire")]
pub struct I18nText {
    /// French.
    pub fr: String,
    /// English.
    pub en: String,
    /// Any other language, keyed by its code.
    #[serde(flatten)]
    pub others: BTreeMap<String, String>,
}

/// What [`I18nText`] reads: an object, or one text for every language.
#[derive(Deserialize)]
#[serde(untagged)]
enum Wire {
    Same(String),
    ByLanguage(BTreeMap<String, Option<String>>),
}

impl From<Wire> for I18nText {
    fn from(wire: Wire) -> Self {
        match wire {
            Wire::Same(text) => Self::new(text.clone(), text),
            Wire::ByLanguage(mut texts) => {
                let mut take =
                    |language: &str| texts.remove(language).flatten().unwrap_or_default();
                let (fr, en) = (take("fr"), take("en"));
                Self {
                    fr,
                    en,
                    others: texts
                        .into_iter()
                        .filter_map(|(language, text)| Some((language, text?)))
                        .collect(),
                }
            }
        }
    }
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

    /// The text for `locale` (`fr-FR` and `fr` alike), else French, else English, else the first
    /// other language that has one; `""` when every language is blank.
    pub fn get(&self, locale: &str) -> &str {
        let language = locale
            .split(['-', '_'])
            .next()
            .unwrap_or(locale)
            .to_ascii_lowercase();
        [self.text(&language), self.text("fr"), self.text("en")]
            .into_iter()
            .flatten()
            .chain(
                self.others
                    .values()
                    .map(String::as_str)
                    .filter(|t| !t.trim().is_empty()),
            )
            .next()
            .unwrap_or("")
    }

    /// What the host form shows: the text in the language of whoever is editing
    /// ([`Context::locale`]), with the fallback of [`Self::get`]. The platform writes the value the
    /// form sends back into that same language, and keeps the others.
    pub fn host_value(&self, ctx: &Context) -> &str {
        self.get(&ctx.locale)
    }

    /// No language has a non-blank text — what the platform calls an empty `localized` field.
    pub fn is_blank(&self) -> bool {
        self.fr.trim().is_empty()
            && self.en.trim().is_empty()
            && self.others.values().all(|text| text.trim().is_empty())
    }

    /// The non-blank text of exactly `language`.
    fn text(&self, language: &str) -> Option<&str> {
        match language {
            "fr" => Some(self.fr.as_str()),
            "en" => Some(self.en.as_str()),
            other => self.others.get(other).map(String::as_str),
        }
        .filter(|text| !text.trim().is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn read(value: serde_json::Value) -> I18nText {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn reads_an_object_or_a_string() {
        assert_eq!(
            read(json!({ "fr": "Salut", "en": "Hi", "de": "Hallo", "it": null })),
            I18nText::new("Salut", "Hi").with("de", "Hallo")
        );
        assert_eq!(
            read(json!({ "de": "Hallo" })),
            I18nText::default().with("de", "Hallo")
        );
        assert_eq!(read(json!("Salut")), I18nText::new("Salut", "Salut"));
        assert!(serde_json::from_value::<I18nText>(json!(3)).is_err());
    }

    #[test]
    fn get_falls_back_to_fr_then_en_then_any() {
        let text = I18nText::new("Salut", "Hi").with("de", "Hallo");
        assert_eq!(text.get("de-DE"), "Hallo");
        assert_eq!(text.get("en_GB"), "Hi");
        assert_eq!(text.get("es"), "Salut");
        assert_eq!(I18nText::new(" ", "Hi").get("es"), "Hi");
        assert_eq!(I18nText::default().with("it", "Ciao").get("fr"), "Ciao");
        assert_eq!(I18nText::default().get("fr"), "");
    }

    #[test]
    fn host_value_is_in_the_editor_language() {
        let text = I18nText::new("Salut", "Hi");
        let ctx = Context {
            locale: "en-US".into(),
            ..Context::default()
        };
        assert_eq!(text.host_value(&ctx), "Hi");
    }

    #[test]
    fn blank_means_no_language_has_text() {
        assert!(I18nText::default().is_blank());
        assert!(I18nText::new(" ", "").with("de", "\n").is_blank());
        assert!(!I18nText::default().with("de", "Hallo").is_blank());
    }
}
