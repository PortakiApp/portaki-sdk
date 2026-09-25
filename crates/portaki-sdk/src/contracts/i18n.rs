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
        let language = crate::context::short_lang(locale).unwrap_or_default();
        self.first_of(&[&language])
    }

    /// What the reader of this invocation sees: the text in their language ([`Context::lang`]),
    /// else in the property's default language ([`Context::property_lang`] — what the host most
    /// likely wrote in), else French, English, the first language that has one.
    ///
    /// ```
    /// use portaki_sdk::context::Context;
    /// use portaki_sdk::contracts::i18n::I18nText;
    ///
    /// let note = I18nText::default().with("es", "Llave en el buzón").with("it", "Chiave");
    /// let ctx = Context {
    ///     locale: "de-DE".into(),
    ///     property_lang: Some("es".into()),
    ///     ..Context::default()
    /// };
    /// assert_eq!(note.for_ctx(&ctx), "Llave en el buzón"); // no German: the property's Spanish
    /// ```
    pub fn for_ctx(&self, ctx: &Context) -> &str {
        let lang = ctx.lang();
        self.first_of(&[&lang, ctx.property_lang().unwrap_or_default()])
    }

    /// The first of `languages` with a text, then French, English, any other language.
    fn first_of(&self, languages: &[&str]) -> &str {
        languages
            .iter()
            .map(|language| self.text(&language.to_ascii_lowercase()))
            .chain([self.text("fr"), self.text("en")])
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

    /// The text of `key` in each bundle that has it, `{name}` placeholders filled from `vars` —
    /// what [`bundle_text!`](crate::bundle_text) calls with the module's embedded bundles.
    ///
    /// `bundles` are `(language, JSON)` pairs, flat `{ "key": "text" }` objects; the first bundle
    /// of a language that has the key wins. A bundle that does not parse is skipped.
    ///
    /// ```
    /// use portaki_sdk::contracts::i18n::I18nText;
    ///
    /// let bundles = [("fr", r#"{"nights":"{n} nuits"}"#), ("en", r#"{"nights":"{n} nights"}"#)];
    /// let text = I18nText::from_bundles(&bundles, "nights", &[("n", "3")]);
    /// assert_eq!(text, I18nText::new("3 nuits", "3 nights"));
    /// ```
    pub fn from_bundles(bundles: &[(&str, &str)], key: &str, vars: &[(&str, &str)]) -> Self {
        let mut text = Self::default();
        for (language, json) in bundles {
            let Ok(serde_json::Value::Object(bundle)) = serde_json::from_str(json) else {
                continue;
            };
            let Some(raw) = bundle.get(key).and_then(serde_json::Value::as_str) else {
                continue;
            };
            let slot = match *language {
                "fr" => &mut text.fr,
                "en" => &mut text.en,
                other => text.others.entry(other.to_string()).or_default(),
            };
            if slot.is_empty() {
                *slot = vars.iter().fold(raw.to_string(), |text, (name, value)| {
                    text.replace(&format!("{{{name}}}"), value)
                });
            }
        }
        text
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

impl From<I18nText> for crate::host::email::LocalizedEmailText {
    /// The same texts, other languages under `translations` — for an email built from
    /// [`bundle_text!`](crate::bundle_text).
    fn from(text: I18nText) -> Self {
        Self {
            fr: text.fr,
            en: text.en,
            translations: text.others,
        }
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
    fn for_ctx_tries_the_reader_then_the_property_language() {
        let text = I18nText::new("", "Hi")
            .with("es", "Hola")
            .with("de", "Hallo");
        let ctx = |locale: &str, property: Option<&str>| Context {
            locale: locale.into(),
            property_lang: property.map(Into::into),
            ..Context::default()
        };
        assert_eq!(text.for_ctx(&ctx("de-DE", Some("es"))), "Hallo");
        assert_eq!(text.for_ctx(&ctx("it-IT", Some("es"))), "Hola");
        assert_eq!(text.for_ctx(&ctx("it-IT", None)), "Hi", "fr blank → en");
        assert_eq!(text.for_ctx(&ctx("it-IT", Some("pt"))), "Hi");
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
    fn from_bundles_takes_the_first_text_per_language() {
        let bundles = [
            ("fr", r#"{"k":"Salut {who}"}"#),
            ("en", "not json"),
            ("en", r#"{"k":"Hi {who}"}"#),
            ("de", r#"{"other":"x"}"#),
            ("fr", r#"{"k":"ignored"}"#),
            ("it", r#"{"k":"Ciao {who}"}"#),
        ];
        let text = I18nText::from_bundles(&bundles, "k", &[("who", "Ada")]);
        assert_eq!(
            text,
            I18nText::new("Salut Ada", "Hi Ada").with("it", "Ciao Ada")
        );
        let email: crate::host::email::LocalizedEmailText = text.into();
        assert_eq!(email.translations["it"], "Ciao Ada");
    }

    #[test]
    fn blank_means_no_language_has_text() {
        assert!(I18nText::default().is_blank());
        assert!(I18nText::new(" ", "").with("de", "\n").is_blank());
        assert!(!I18nText::default().with("de", "Hallo").is_blank());
    }
}
