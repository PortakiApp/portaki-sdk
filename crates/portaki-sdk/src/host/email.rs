//! `host::email` — ask the orchestrator to send a transactional email.
//!
//! Modules own the email **content** (subject / title / body SDUI payload). The
//! gateway wraps it in the guest (or host) Thymeleaf shell + property brand.
//!
//! Wire op: `email.send`. Runtime forwards the request as platform event
//! [`crate::contracts::platform::EMAIL_SEND`] so the orchestrator can resolve
//! recipients and render.
//!
//! ## Localized copy
//!
//! [`LocalizedEmailText`] is wire-compatible with the historical `{ "fr", "en" }`
//! shape. Extra locales live in [`LocalizedEmailText::translations`] (e.g.
//! `"de"`, `"es"`). Resolution order:
//! `guestLang → language tag → en → fr → first non-blank`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::error::Result;
use crate::host::runtime::backend;

/// Who should receive the mail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EmailAudience {
    /// Guest stay recipient (`stayId` or current guest session).
    Guest,
    /// Workspace owner (host dashboard).
    Host,
    /// Fan-out to UPCOMING (≤24h) + ACTIVE stays on the property.
    PropertyEligibleGuests,
}

/// Localized string used in module email payloads (guest + host).
///
/// Wire: `{ "fr": "…", "en": "…", "translations": { "de": "…", … } }`.
/// Legacy payloads without `translations` still deserialize.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct LocalizedEmailText {
    /// French copy (legacy primary).
    #[serde(default)]
    pub fr: String,
    /// English copy (legacy secondary).
    #[serde(default)]
    pub en: String,
    /// Additional locales keyed by language tag (`de`, `es`, `zh`, …).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub translations: BTreeMap<String, String>,
}

impl LocalizedEmailText {
    /// Builds from FR + EN (host-audience / legacy).
    pub fn new(fr: impl Into<String>, en: impl Into<String>) -> Self {
        Self {
            fr: fr.into(),
            en: en.into(),
            translations: BTreeMap::new(),
        }
    }

    /// Same string for FR and EN.
    pub fn both(text: impl Into<String>) -> Self {
        let text = text.into();
        Self {
            fr: text.clone(),
            en: text,
            translations: BTreeMap::new(),
        }
    }

    /// Builds from an explicit locale → text map (`fr` / `en` promoted to fields).
    pub fn from_map(map: BTreeMap<String, String>) -> Self {
        let mut translations = BTreeMap::new();
        let mut fr = String::new();
        let mut en = String::new();
        for (locale, text) in map {
            let code = normalize_lang_tag(&locale);
            if code.is_empty() || text.is_empty() {
                continue;
            }
            match code.as_str() {
                "fr" => fr = text,
                "en" => en = text,
                _ => {
                    translations.insert(code, text);
                }
            }
        }
        Self {
            fr,
            en,
            translations,
        }
    }

    /// Looks up `key` in each locale bundle and builds a multi-locale text.
    ///
    /// `bundles` entries are `(locale, flat key→string JSON object)`.
    /// Missing keys are skipped. Supports `{var}` interpolation via `vars`.
    pub fn from_i18n_key(
        bundles: impl IntoIterator<Item = (impl AsRef<str>, &'static str)>,
        key: &str,
    ) -> Self {
        Self::from_i18n_key_with_vars(bundles, key, &[])
    }

    /// Same as [`from_i18n_key`] with `{name}` → value substitution.
    pub fn from_i18n_key_with_vars(
        bundles: impl IntoIterator<Item = (impl AsRef<str>, &'static str)>,
        key: &str,
        vars: &[(&str, &str)],
    ) -> Self {
        let mut map = BTreeMap::new();
        for (locale, json) in bundles {
            let Ok(value) = serde_json::from_str::<Value>(json) else {
                continue;
            };
            let Some(obj) = value.as_object() else {
                continue;
            };
            let Some(raw) = obj.get(key).and_then(|v| v.as_str()) else {
                continue;
            };
            let text = interpolate(raw, vars);
            if !text.is_empty() {
                map.insert(normalize_lang_tag(locale.as_ref()), text);
            }
        }
        Self::from_map(map)
    }

    /// Resolves copy for `locale` with the guest-email fallback chain.
    pub fn resolve(&self, locale: &str) -> &str {
        for candidate in locale_fallback_chain(locale) {
            if let Some(text) = self.get_raw(&candidate) {
                if !text.is_empty() {
                    return text;
                }
            }
        }
        self.first_non_blank()
    }

    /// Alias used by Java-side naming (`forLocale`).
    pub fn for_locale(&self, locale: &str) -> &str {
        self.resolve(locale)
    }

    fn get_raw(&self, lang: &str) -> Option<&str> {
        match lang {
            "fr" => Some(self.fr.as_str()),
            "en" => Some(self.en.as_str()),
            other => self.translations.get(other).map(String::as_str),
        }
    }

    fn first_non_blank(&self) -> &str {
        if !self.en.is_empty() {
            return self.en.as_str();
        }
        if !self.fr.is_empty() {
            return self.fr.as_str();
        }
        for text in self.translations.values() {
            if !text.is_empty() {
                return text.as_str();
            }
        }
        ""
    }
}

/// Normalizes `zh-CN` / `EN` / ` fr ` → language tag (`zh`, `en`, `fr`).
pub fn normalize_lang_tag(raw: &str) -> String {
    let trimmed = raw.trim().to_ascii_lowercase();
    if trimmed.is_empty() {
        return String::new();
    }
    trimmed
        .split(['-', '_'])
        .next()
        .unwrap_or("")
        .to_string()
}

/// Fallback chain: guestLang → language tag → en → fr.
pub fn locale_fallback_chain(guest_lang: &str) -> Vec<String> {
    let mut out = Vec::with_capacity(4);
    let trimmed = guest_lang.trim().to_ascii_lowercase();
    if !trimmed.is_empty() {
        out.push(trimmed.clone());
        let tag = normalize_lang_tag(&trimmed);
        if !tag.is_empty() && tag != trimmed {
            out.push(tag);
        }
    }
    for fallback in ["en", "fr"] {
        if !out.iter().any(|s| s == fallback) {
            out.push(fallback.to_string());
        }
    }
    out
}

fn interpolate(template: &str, vars: &[(&str, &str)]) -> String {
    let mut text = template.to_string();
    for (name, value) in vars {
        text = text.replace(&format!("{{{name}}}"), value);
    }
    text
}

/// Optional CTA rendered in the module-transactional shell.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ModuleEmailCta {
    /// Button label.
    pub label: LocalizedEmailText,
    /// Absolute URL, or guest booklet URL when no portaki action.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Guest booklet deep-link action (preferred over raw URL when set).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub portaki_action: Option<String>,
}

/// Module-owned email body (email SDUI / content contract).
///
/// Rendered inside `_base-guest` (or host shell) via Thymeleaf
/// `module-transactional`. Keep copy and structure here — not in orchestrator
/// Java.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ModuleEmailSdui {
    /// Subject line.
    pub subject: LocalizedEmailText,
    /// Optional eyebrow above the title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eyebrow: Option<LocalizedEmailText>,
    /// Optional H1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<LocalizedEmailText>,
    /// Body paragraphs — separate with blank lines (`\n\n`).
    pub body: LocalizedEmailText,
    /// Optional CTA.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cta: Option<ModuleEmailCta>,
}

/// Arguments for [`send`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SendEmailArgs {
    /// Stable id for logs / delivery dedup (module-scoped), e.g. `host-found`.
    pub email_id: String,
    /// Recipient strategy.
    pub audience: EmailAudience,
    /// Module-owned content.
    pub content: ModuleEmailSdui,
    /// Target stay — required for [`EmailAudience::Guest`] when not in guest session.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stay_id: Option<Uuid>,
    /// Property scope — required for [`EmailAudience::PropertyEligibleGuests`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub property_id: Option<Uuid>,
    /// Host dashboard deep-link (host audience).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_url: Option<String>,
}

/// Asks the orchestrator to send `args` (guest shell + brand, or host shell).
pub fn send(args: &SendEmailArgs) -> Result<()> {
    let payload_json = serde_json::to_string(args)?;
    backend()?.email_send(&payload_json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_fr_en_roundtrip() {
        let text = LocalizedEmailText::new("Bonjour", "Hello");
        let json = serde_json::to_string(&text).unwrap();
        assert!(json.contains("\"fr\":\"Bonjour\""));
        assert!(json.contains("\"en\":\"Hello\""));
        assert!(!json.contains("translations"));
        let back: LocalizedEmailText = serde_json::from_str(&json).unwrap();
        assert_eq!(back, text);
    }

    #[test]
    fn resolve_fallback_chain() {
        let mut text = LocalizedEmailText::new("FR", "EN");
        text.translations.insert("de".into(), "DE".into());
        assert_eq!(text.resolve("de"), "DE");
        assert_eq!(text.resolve("de-DE"), "DE");
        assert_eq!(text.resolve("es"), "EN");
        assert_eq!(text.resolve("unknown"), "EN");
        let fr_only = LocalizedEmailText::new("FR", "");
        assert_eq!(fr_only.resolve("es"), "FR");
    }

    #[test]
    fn from_i18n_key_builds_map() {
        let en = r#"{"email.subject":"Hello {name}"}"#;
        let fr = r#"{"email.subject":"Bonjour {name}"}"#;
        let de = r#"{"email.subject":"Hallo {name}"}"#;
        let text = LocalizedEmailText::from_i18n_key_with_vars(
            [("en", en), ("fr", fr), ("de", de)],
            "email.subject",
            &[("name", "Ada")],
        );
        assert_eq!(text.en, "Hello Ada");
        assert_eq!(text.fr, "Bonjour Ada");
        assert_eq!(text.translations.get("de").map(String::as_str), Some("Hallo Ada"));
        assert_eq!(text.resolve("de"), "Hallo Ada");
    }

    #[test]
    fn locale_fallback_chain_order() {
        assert_eq!(
            locale_fallback_chain("zh-CN"),
            vec!["zh-cn".to_string(), "zh".to_string(), "en".to_string(), "fr".to_string()]
        );
        assert_eq!(
            locale_fallback_chain("en"),
            vec!["en".to_string(), "fr".to_string()]
        );
    }
}
