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
//!
//! ## Limits
//!
//! The platform enforces every value in [`crate::limits`]; [`send`] checks the ones it can
//! before calling the host, so a violation fails `cargo test` instead of being dropped
//! silently by the orchestrator in production.
//!
//! | Rule | Checked by the SDK | Error |
//! |------|--------------------|-------|
//! | `email_id`, subject and body not blank | yes | [`EmailError::EmptyField`] |
//! | Subject ≤ 200, eyebrow ≤ 120, title ≤ 200, body ≤ 5000, CTA label ≤ 80 chars, per locale | yes | [`EmailError::FieldTooLong`] |
//! | `action_url` is `https` | yes | [`EmailError::ActionUrlNotHttps`] |
//! | `action_url` on the Portaki web origin | no — the platform drops the link otherwise | — |
//! | Guest email refused once `checkout_at + 7 days` has passed | when the target is the invocation stay | [`EmailError::StayEnded`] (`email_stay_ended`) |
//! | ≤ 5 `email.send` per invocation | host (mocked by `portaki-test-utils`) | [`EmailError::LimitExceeded`] (`email_limit_exceeded`) |
//! | ≤ 3 module emails per guest stay per rolling 24 h, ≤ 10 per stay, all modules | no — platform only | dropped |
//! | ≤ 20 host emails per module per workspace per rolling 24 h | no — platform only | dropped |
//! | ≤ 10 blocks, 1–12 items per block (`stats`: 2 or 4), required texts not blank | yes | [`EmailError::InvalidBlock`] |
//! | Every block text ≤ 200 chars per locale | yes | [`EmailError::InvalidBlock`] (`text_too_long`) |
//! | Block emoji: one short symbol (≤ 16 UTF-16 units, no ASCII, letter, digit or space) | yes; the platform also requires a single grapheme | [`EmailError::InvalidBlock`] |
//! | No link written in any text (`scheme://`, `www.`) — links go through the CTA only | no — platform only | dropped |
//! | Unknown block type, tone or hero | cannot be built with these types | dropped |
//!
//! ## Blocks
//!
//! Beyond the paragraphs of `body`, [`ModuleEmailSdui::blocks`] carries structured blocks the
//! platform renders with its own templates — a checklist, address cards, a timetable, a form
//! preview, a receipt, figures, label/value rows, details, a note — and
//! [`ModuleEmailSdui::hero`] picks the header: plain intro, dark tile holding the button, or a
//! toned alert tile. Every text is plain text (never HTML). A block outside the contract makes
//! the platform refuse the whole email, never send it without the block.
//!
//! ```
//! use portaki_sdk::host::email::{
//!     EmailBlock, EmailHero, EmailPair, EmailTone, LocalizedEmailText as T, ModuleEmailSdui,
//! };
//!
//! let content = ModuleEmailSdui {
//!     subject: T::new("Synchronisation échouée", "Sync failed"),
//!     body: T::new("Nous n’arrivons plus à lire votre calendrier.", "We can no longer read your calendar."),
//!     hero: EmailHero::Alert { tone: EmailTone::Warning },
//!     blocks: vec![
//!         EmailBlock::rows([EmailPair::new(T::both("Source"), T::both("Airbnb · iCal"))]),
//!         EmailBlock::note(EmailTone::Info, T::new("Vérifiez le lien iCal.", "Check the iCal link.")),
//!     ],
//!     ..Default::default()
//! };
//! assert_eq!(content.blocks.len(), 2);
//! ```

use std::collections::BTreeMap;
use std::fmt;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::context::Context;
#[cfg(feature = "email")]
use crate::error::{PortakiError, Result};
#[cfg(feature = "email")]
use crate::host::runtime::{backend, context_or_load};
use crate::limits;

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
    trimmed.split(['-', '_']).next().unwrap_or("").to_string()
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

/// Tone of a [`EmailBlock::Note`] or an [`EmailHero::Alert`] — the platform's closed list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EmailTone {
    /// Property / Portaki colour.
    Brand,
    /// Green.
    Success,
    /// Blue.
    Info,
    /// Amber.
    Warning,
    /// Red.
    Danger,
    /// Grey.
    Neutral,
}

/// Header of a module email. Wire: `{ "kind": "intro" | "ink" | "alert", "tone"? }`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum EmailHero {
    /// Eyebrow, title, greeting and body paragraphs on the page (default, not sent on the wire).
    #[default]
    Intro,
    /// Dark tile: title, greeting, paragraphs and the CTA button inside the tile.
    Ink,
    /// Tile coloured by `tone`, eyebrow with a dot — for failures and warnings.
    Alert {
        /// Colour of the tile.
        tone: EmailTone,
    },
}

impl EmailHero {
    fn is_intro(&self) -> bool {
        *self == Self::Intro
    }
}

/// A `label` / `value` line of [`EmailBlock::Receipt`], [`EmailBlock::Stats`],
/// [`EmailBlock::Rows`] and [`EmailBlock::Details`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct EmailPair {
    /// Left-hand label.
    pub label: LocalizedEmailText,
    /// Right-hand value.
    pub value: LocalizedEmailText,
}

impl EmailPair {
    /// `label` → `value`.
    pub fn new(label: LocalizedEmailText, value: LocalizedEmailText) -> Self {
        Self { label, value }
    }
}

/// An address card of [`EmailBlock::Places`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct EmailPlace {
    /// One emoji (`"🥐"`), shown in a square.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emoji: Option<String>,
    /// Name of the place.
    pub name: LocalizedEmailText,
    /// Small line under the name (`Boulangerie · 800 m`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<LocalizedEmailText>,
}

impl EmailPlace {
    /// A card with just a name.
    pub fn new(name: LocalizedEmailText) -> Self {
        Self {
            name,
            ..Self::default()
        }
    }

    /// Sets the emoji.
    pub fn with_emoji(mut self, emoji: impl Into<String>) -> Self {
        self.emoji = Some(emoji.into());
        self
    }

    /// Sets the line under the name.
    pub fn with_meta(mut self, meta: LocalizedEmailText) -> Self {
        self.meta = Some(meta);
        self
    }
}

/// A line of [`EmailBlock::Timetable`]: a time in large figures, then its line.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct EmailTimeLine {
    /// `08:12`.
    pub time: LocalizedEmailText,
    /// `Antibes → Nice-Ville`.
    pub text: LocalizedEmailText,
}

impl EmailTimeLine {
    /// `time` then `text`.
    pub fn new(time: LocalizedEmailText, text: LocalizedEmailText) -> Self {
        Self { time, text }
    }
}

/// A field previewed by [`EmailBlock::Fields`]: its label over an empty dashed box.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct EmailFormField {
    /// Field label.
    pub label: LocalizedEmailText,
    /// Greyed text in the box (`À renseigner`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<LocalizedEmailText>,
}

impl EmailFormField {
    /// A field with no hint.
    pub fn new(label: LocalizedEmailText) -> Self {
        Self { label, hint: None }
    }

    /// Sets the greyed hint.
    pub fn with_hint(mut self, hint: LocalizedEmailText) -> Self {
        self.hint = Some(hint);
        self
    }
}

/// A structured block of a module email, rendered by the platform's templates.
///
/// Wire: `{ "type": "checklist", … }`. Every text is plain text. Build them with the
/// constructors ([`EmailBlock::checklist`], [`EmailBlock::rows`], …) and
/// [`EmailBlock::labeled`]; limits in [`crate::limits`] (`EMAIL_BLOCK_*`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum EmailBlock {
    /// Tile of check boxes, one per item.
    Checklist {
        /// Small heading above the block.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<LocalizedEmailText>,
        /// One line per box.
        items: Vec<LocalizedEmailText>,
    },
    /// Address cards, three per row on desktop, stacked on mobile.
    Places {
        /// Small heading above the block.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<LocalizedEmailText>,
        /// The cards.
        items: Vec<EmailPlace>,
    },
    /// Times in large figures, each with its line.
    Timetable {
        /// Heading inside the tile.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<LocalizedEmailText>,
        /// The lines.
        items: Vec<EmailTimeLine>,
    },
    /// Preview of a form: each field's label over an empty box.
    Fields {
        /// Small heading above the block.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<LocalizedEmailText>,
        /// The fields.
        items: Vec<EmailFormField>,
    },
    /// Receipt card: header, lines, optional total.
    #[serde(rename_all = "camelCase")]
    Receipt {
        /// Header, left.
        label: LocalizedEmailText,
        /// Header, right.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        meta: Option<LocalizedEmailText>,
        /// The lines.
        items: Vec<EmailPair>,
        /// Label of the total — set with `total`, or neither.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        total_label: Option<LocalizedEmailText>,
        /// The total, in large figures.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        total: Option<LocalizedEmailText>,
    },
    /// Two or four figures in tiles, two per row.
    Stats {
        /// First tile dark.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        ink: bool,
        /// Exactly 2 or 4.
        items: Vec<EmailPair>,
    },
    /// Label / value lines.
    Rows {
        /// Small heading above the block.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<LocalizedEmailText>,
        /// The lines.
        items: Vec<EmailPair>,
    },
    /// Label / value lines in a grey box (device, source, error…).
    Details {
        /// Heading of the box.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<LocalizedEmailText>,
        /// The lines.
        items: Vec<EmailPair>,
    },
    /// Callout after a rule: a dot of `tone` with its label, then the text.
    Note {
        /// Colour of the dot and label.
        tone: EmailTone,
        /// Label next to the dot.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<LocalizedEmailText>,
        /// The text.
        text: LocalizedEmailText,
    },
}

impl EmailBlock {
    /// Check boxes.
    pub fn checklist(items: impl IntoIterator<Item = LocalizedEmailText>) -> Self {
        Self::Checklist {
            label: None,
            items: items.into_iter().collect(),
        }
    }

    /// Address cards.
    pub fn places(items: impl IntoIterator<Item = EmailPlace>) -> Self {
        Self::Places {
            label: None,
            items: items.into_iter().collect(),
        }
    }

    /// Times and their lines.
    pub fn timetable(items: impl IntoIterator<Item = EmailTimeLine>) -> Self {
        Self::Timetable {
            label: None,
            items: items.into_iter().collect(),
        }
    }

    /// Form preview.
    pub fn fields(items: impl IntoIterator<Item = EmailFormField>) -> Self {
        Self::Fields {
            label: None,
            items: items.into_iter().collect(),
        }
    }

    /// Receipt without total — add one with [`EmailBlock::with_total`].
    pub fn receipt(label: LocalizedEmailText, items: impl IntoIterator<Item = EmailPair>) -> Self {
        Self::Receipt {
            label,
            meta: None,
            items: items.into_iter().collect(),
            total_label: None,
            total: None,
        }
    }

    /// Two or four figures; [`EmailBlock::ink`] darkens the first.
    pub fn stats(items: impl IntoIterator<Item = EmailPair>) -> Self {
        Self::Stats {
            ink: false,
            items: items.into_iter().collect(),
        }
    }

    /// Label / value lines.
    pub fn rows(items: impl IntoIterator<Item = EmailPair>) -> Self {
        Self::Rows {
            label: None,
            items: items.into_iter().collect(),
        }
    }

    /// Label / value lines in a grey box.
    pub fn details(items: impl IntoIterator<Item = EmailPair>) -> Self {
        Self::Details {
            label: None,
            items: items.into_iter().collect(),
        }
    }

    /// Callout.
    pub fn note(tone: EmailTone, text: LocalizedEmailText) -> Self {
        Self::Note {
            tone,
            label: None,
            text,
        }
    }

    /// Sets the heading of the block (the header of a receipt); no effect on `stats`.
    pub fn labeled(mut self, text: LocalizedEmailText) -> Self {
        match &mut self {
            Self::Checklist { label, .. }
            | Self::Places { label, .. }
            | Self::Timetable { label, .. }
            | Self::Fields { label, .. }
            | Self::Rows { label, .. }
            | Self::Details { label, .. }
            | Self::Note { label, .. } => *label = Some(text),
            Self::Receipt { label, .. } => *label = text,
            Self::Stats { .. } => {}
        }
        self
    }

    /// Sets the right-hand header of a receipt; no effect on other blocks.
    pub fn with_meta(mut self, text: LocalizedEmailText) -> Self {
        if let Self::Receipt { meta, .. } = &mut self {
            *meta = Some(text);
        }
        self
    }

    /// Sets the total of a receipt; no effect on other blocks.
    pub fn with_total(mut self, label: LocalizedEmailText, value: LocalizedEmailText) -> Self {
        if let Self::Receipt {
            total_label, total, ..
        } = &mut self
        {
            *total_label = Some(label);
            *total = Some(value);
        }
        self
    }

    /// Darkens the first figure of `stats`; no effect on other blocks.
    pub fn ink(mut self) -> Self {
        if let Self::Stats { ink, .. } = &mut self {
            *ink = true;
        }
        self
    }

    /// Wire name of the block type.
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Checklist { .. } => "checklist",
            Self::Places { .. } => "places",
            Self::Timetable { .. } => "timetable",
            Self::Fields { .. } => "fields",
            Self::Receipt { .. } => "receipt",
            Self::Stats { .. } => "stats",
            Self::Rows { .. } => "rows",
            Self::Details { .. } => "details",
            Self::Note { .. } => "note",
        }
    }

    /// `(required texts, optional texts, item count, emojis)` — what [`Self::check`] judges.
    fn parts(
        &self,
    ) -> (
        Vec<&LocalizedEmailText>,
        Vec<&LocalizedEmailText>,
        Option<usize>,
        Vec<&str>,
    ) {
        fn pairs(items: &[EmailPair]) -> Vec<&LocalizedEmailText> {
            items.iter().flat_map(|p| [&p.label, &p.value]).collect()
        }
        match self {
            Self::Checklist { label, items } => (
                items.iter().collect(),
                label.iter().collect(),
                Some(items.len()),
                vec![],
            ),
            Self::Places { label, items } => (
                items.iter().map(|p| &p.name).collect(),
                label
                    .iter()
                    .chain(items.iter().filter_map(|p| p.meta.as_ref()))
                    .collect(),
                Some(items.len()),
                items.iter().filter_map(|p| p.emoji.as_deref()).collect(),
            ),
            Self::Timetable { label, items } => (
                items.iter().flat_map(|l| [&l.time, &l.text]).collect(),
                label.iter().collect(),
                Some(items.len()),
                vec![],
            ),
            Self::Fields { label, items } => (
                items.iter().map(|f| &f.label).collect(),
                label
                    .iter()
                    .chain(items.iter().filter_map(|f| f.hint.as_ref()))
                    .collect(),
                Some(items.len()),
                vec![],
            ),
            Self::Receipt {
                label,
                meta,
                items,
                total_label,
                total,
            } => {
                let mut required = pairs(items);
                required.push(label);
                (
                    required,
                    meta.iter().chain(total_label).chain(total).collect(),
                    Some(items.len()),
                    vec![],
                )
            }
            Self::Stats { items, .. } => (pairs(items), vec![], Some(items.len()), vec![]),
            Self::Rows { label, items } | Self::Details { label, items } => (
                pairs(items),
                label.iter().collect(),
                Some(items.len()),
                vec![],
            ),
            Self::Note { label, text, .. } => (vec![text], label.iter().collect(), None, vec![]),
        }
    }

    /// The platform's rules for one block, as far as the SDK can judge them.
    fn check(&self, index: usize) -> std::result::Result<(), EmailError> {
        let invalid = |reason| EmailError::InvalidBlock {
            index,
            kind: self.kind(),
            reason,
        };
        let (required, optional, count, emojis) = self.parts();
        if let Some(count) = count {
            let (min, max) = match self {
                Self::Stats { .. } => (2, 4),
                _ => (1, limits::EMAIL_BLOCK_ITEMS_MAX),
            };
            if count < min || count > max || (matches!(self, Self::Stats { .. }) && count % 2 != 0)
            {
                return Err(invalid("item_count"));
            }
        }
        if let Self::Receipt {
            total_label, total, ..
        } = self
        {
            if total_label.is_some() != total.is_some() {
                return Err(invalid("total_and_label_together"));
            }
        }
        if required.iter().any(|text| text.is_blank()) {
            return Err(invalid("required_text_blank"));
        }
        let too_long = required.iter().chain(&optional).any(|text| {
            text.entries()
                .any(|(_, copy)| copy.chars().count() > limits::EMAIL_BLOCK_TEXT_MAX_CHARS)
        });
        if too_long {
            return Err(invalid("text_too_long"));
        }
        if !emojis.into_iter().all(is_block_emoji) {
            return Err(invalid("not_an_emoji"));
        }
        Ok(())
    }
}

/// One short symbol: ≤ [`limits::EMAIL_BLOCK_EMOJI_MAX_UTF16`] UTF-16 units, nothing ASCII,
/// alphanumeric, blank or control. The platform additionally requires a single grapheme.
fn is_block_emoji(emoji: &str) -> bool {
    let units = emoji.encode_utf16().count();
    (1..=limits::EMAIL_BLOCK_EMOJI_MAX_UTF16).contains(&units)
        && !emoji
            .chars()
            .any(|c| c.is_ascii() || c.is_alphanumeric() || c.is_whitespace() || c.is_control())
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
    /// Header: plain intro (default), dark tile holding the CTA, or toned alert tile.
    #[serde(default, skip_serializing_if = "EmailHero::is_intro")]
    pub hero: EmailHero,
    /// Structured blocks after the body paragraphs, before the CTA — see the
    /// [module docs](self#blocks).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<EmailBlock>,
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

/// Field of a [`SendEmailArgs`] named by an [`EmailError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum EmailField {
    /// [`SendEmailArgs::email_id`].
    EmailId,
    /// [`ModuleEmailSdui::subject`].
    Subject,
    /// [`ModuleEmailSdui::eyebrow`].
    Eyebrow,
    /// [`ModuleEmailSdui::title`].
    Title,
    /// [`ModuleEmailSdui::body`].
    Body,
    /// [`ModuleEmailCta::label`].
    CtaLabel,
}

impl EmailField {
    /// Wire path of the field in the `email.send` payload.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EmailId => "emailId",
            Self::Subject => "content.subject",
            Self::Eyebrow => "content.eyebrow",
            Self::Title => "content.title",
            Self::Body => "content.body",
            Self::CtaLabel => "content.cta.label",
        }
    }
}

impl fmt::Display for EmailField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Why an email was refused — see the [module docs](self#limits) for the full rule table.
///
/// Each variant has a stable machine [`code`](Self::code); the ones the platform also
/// returns (`email_stay_ended`, `email_limit_exceeded`) use the platform's code verbatim.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum EmailError {
    /// A required field is blank (`email_id`, or every locale of subject / body).
    #[error("email_field_empty: {field} must not be blank")]
    EmptyField {
        /// The blank field.
        field: EmailField,
    },

    /// One locale of a copy field is longer than its cap in [`crate::limits`].
    #[error("email_field_too_long: {field} ({locale}) has {actual} chars, max {max}")]
    FieldTooLong {
        /// The field over its cap.
        field: EmailField,
        /// Locale of the offending copy (`fr`, `en`, or a translation tag).
        locale: String,
        /// Its length, in chars.
        actual: usize,
        /// The cap, in chars.
        max: usize,
    },

    /// A block breaks the platform's contract: item count, blank required text, text over
    /// [`limits::EMAIL_BLOCK_TEXT_MAX_CHARS`], total without its label, emoji that is not one.
    #[error("email_block_invalid: blocks[{index}] ({kind}): {reason}")]
    InvalidBlock {
        /// Position in [`ModuleEmailSdui::blocks`].
        index: usize,
        /// Wire type of the block.
        kind: &'static str,
        /// `item_count`, `required_text_blank`, `text_too_long`, `total_and_label_together`,
        /// `not_an_emoji`.
        reason: &'static str,
    },

    /// More than [`limits::EMAIL_BLOCKS_MAX`] blocks.
    #[error("email_too_many_blocks: at most {} blocks", limits::EMAIL_BLOCKS_MAX)]
    TooManyBlocks,

    /// `action_url` is not an absolute `https://` URL.
    ///
    /// The platform additionally requires the Portaki web origin and drops the link
    /// otherwise; the SDK does not know that origin per environment, so it only checks
    /// the scheme.
    #[error("email_action_url_not_https: action_url must be an absolute https URL")]
    ActionUrlNotHttps,

    /// Guest email for a stay whose checkout is more than
    /// [`limits::GUEST_EMAIL_DAYS_AFTER_CHECKOUT`] days in the past.
    #[error(
        "email_stay_ended: guest emails stop {} days after checkout",
        limits::GUEST_EMAIL_DAYS_AFTER_CHECKOUT
    )]
    StayEnded,

    /// More than [`limits::EMAIL_SENDS_PER_INVOCATION`] `email.send` in this invocation.
    #[error(
        "email_limit_exceeded: at most {} email sends per invocation",
        limits::EMAIL_SENDS_PER_INVOCATION
    )]
    LimitExceeded,
}

impl EmailError {
    /// Host code for [`EmailError::StayEnded`].
    pub const STAY_ENDED_CODE: &'static str = "email_stay_ended";
    /// Host code for [`EmailError::LimitExceeded`].
    pub const LIMIT_EXCEEDED_CODE: &'static str = "email_limit_exceeded";

    /// Stable machine code, matching the platform's where it has one.
    pub const fn code(&self) -> &'static str {
        match self {
            Self::EmptyField { .. } => "email_field_empty",
            Self::FieldTooLong { .. } => "email_field_too_long",
            Self::ActionUrlNotHttps => "email_action_url_not_https",
            Self::InvalidBlock { .. } => "email_block_invalid",
            Self::TooManyBlocks => "email_too_many_blocks",
            Self::StayEnded => Self::STAY_ENDED_CODE,
            Self::LimitExceeded => Self::LIMIT_EXCEEDED_CODE,
        }
    }

    /// The variant for a host dispatch error code, when it is an email one.
    pub fn from_host_code(code: &str) -> Option<Self> {
        match code {
            Self::STAY_ENDED_CODE => Some(Self::StayEnded),
            Self::LIMIT_EXCEEDED_CODE => Some(Self::LimitExceeded),
            _ => None,
        }
    }
}

impl LocalizedEmailText {
    /// Every `(locale, copy)` pair, legacy fields first.
    fn entries(&self) -> impl Iterator<Item = (&str, &str)> {
        [("fr", self.fr.as_str()), ("en", self.en.as_str())]
            .into_iter()
            .chain(
                self.translations
                    .iter()
                    .map(|(locale, text)| (locale.as_str(), text.as_str())),
            )
    }

    fn is_blank(&self) -> bool {
        self.entries().all(|(_, text)| text.trim().is_empty())
    }

    fn check_max_chars(
        &self,
        field: EmailField,
        max: usize,
    ) -> std::result::Result<(), EmailError> {
        for (locale, text) in self.entries() {
            // In characters, not in bytes: that is how the platform counts, and a text with
            // accents must not hit the cap before its ASCII equivalent does.
            let actual = text.chars().count();
            if actual > max {
                return Err(EmailError::FieldTooLong {
                    field,
                    locale: locale.to_string(),
                    actual,
                    max,
                });
            }
        }
        Ok(())
    }
}

impl SendEmailArgs {
    /// Checks everything about `self` the SDK can know without the host.
    ///
    /// [`send`] calls it first; it is public so a module can validate copy built from
    /// host-entered config before persisting it, rather than at send time.
    pub fn validate(&self) -> std::result::Result<(), EmailError> {
        if self.email_id.trim().is_empty() {
            return Err(EmailError::EmptyField {
                field: EmailField::EmailId,
            });
        }
        let content = &self.content;
        if content.subject.is_blank() {
            return Err(EmailError::EmptyField {
                field: EmailField::Subject,
            });
        }
        if content.body.is_blank() {
            return Err(EmailError::EmptyField {
                field: EmailField::Body,
            });
        }
        content
            .subject
            .check_max_chars(EmailField::Subject, limits::EMAIL_SUBJECT_MAX_CHARS)?;
        if let Some(eyebrow) = &content.eyebrow {
            eyebrow.check_max_chars(EmailField::Eyebrow, limits::EMAIL_EYEBROW_MAX_CHARS)?;
        }
        if let Some(title) = &content.title {
            title.check_max_chars(EmailField::Title, limits::EMAIL_TITLE_MAX_CHARS)?;
        }
        content
            .body
            .check_max_chars(EmailField::Body, limits::EMAIL_BODY_MAX_CHARS)?;
        if let Some(cta) = &content.cta {
            cta.label
                .check_max_chars(EmailField::CtaLabel, limits::EMAIL_CTA_LABEL_MAX_CHARS)?;
        }
        if content.blocks.len() > limits::EMAIL_BLOCKS_MAX {
            return Err(EmailError::TooManyBlocks);
        }
        for (index, block) in content.blocks.iter().enumerate() {
            block.check(index)?;
        }
        if let Some(url) = &self.action_url {
            if !is_absolute_https(url) {
                return Err(EmailError::ActionUrlNotHttps);
            }
        }
        Ok(())
    }

    /// Whether the platform would refuse `self` as sent after the stay ended, judged from
    /// `ctx` at instant `now`.
    ///
    /// Only answers `true` for a [`EmailAudience::Guest`] email aimed at the invocation's
    /// own stay (`stay_id` unset or equal to `ctx.stay`) with a known checkout: that is
    /// the only checkout the SDK sees. Any other stay is left to the platform.
    pub fn is_after_stay(&self, ctx: &Context, now: DateTime<Utc>) -> bool {
        self.invocation_stay_checkout(ctx).is_some_and(|checkout| {
            checkout + Duration::days(limits::GUEST_EMAIL_DAYS_AFTER_CHECKOUT) < now
        })
    }

    fn invocation_stay_checkout(&self, ctx: &Context) -> Option<DateTime<Utc>> {
        if self.audience != EmailAudience::Guest {
            return None;
        }
        let stay = ctx.stay.as_ref()?;
        if self.stay_id.is_some_and(|target| target != stay.stay_id) {
            return None;
        }
        stay.checkout_at
    }
}

/// `https://` followed by a non-empty authority; the scheme is case-insensitive.
fn is_absolute_https(url: &str) -> bool {
    let Some(scheme) = url.get(..8) else {
        return false;
    };
    if !scheme.eq_ignore_ascii_case("https://") {
        return false;
    }
    let authority = url[8..].split(['/', '?', '#']).next().unwrap_or("");
    !authority.is_empty() && !authority.chars().any(char::is_whitespace)
}

/// Asks the orchestrator to send `args` (guest shell + brand, or host shell).
///
/// Refuses before reaching the host when [`SendEmailArgs::validate`] fails, or when a guest
/// email targets the invocation stay more than
/// [`limits::GUEST_EMAIL_DAYS_AFTER_CHECKOUT`] days after its checkout. Host refusals with a
/// known code come back as [`PortakiError::Email`] too. See the [module docs](self#limits).
#[cfg(feature = "email")]
pub fn send(args: &SendEmailArgs) -> Result<()> {
    args.validate()?;
    let backend = backend()?;
    if let Some(checkout) = context_or_load()
        .ok()
        .and_then(|ctx| args.invocation_stay_checkout(&ctx))
    {
        // The clock is only asked of the host when the rule can apply. A backend without a
        // clock (a minimal mock) does not block the send: the platform refuses anyway.
        if let Ok(now) = crate::host::time::now() {
            if checkout + Duration::days(limits::GUEST_EMAIL_DAYS_AFTER_CHECKOUT) < now {
                return Err(EmailError::StayEnded.into());
            }
        }
    }
    let payload_json = serde_json::to_string(args)?;
    backend
        .email_send(&payload_json)
        .map_err(PortakiError::typed)
}

#[cfg(test)]
mod guard_tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::context::StayContext;
    use crate::host::runtime::{with_host, HostBackend};

    /// Records what reaches the host, answers a fixed clock, and can refuse with a code.
    struct RecordingHost {
        now: DateTime<Utc>,
        sent: Mutex<Vec<String>>,
        refuse_with: Option<&'static str>,
    }

    impl RecordingHost {
        fn at(now: &str) -> Arc<Self> {
            Arc::new(Self {
                now: instant(now),
                sent: Mutex::new(Vec::new()),
                refuse_with: None,
            })
        }

        fn sent(&self) -> usize {
            self.sent.lock().unwrap().len()
        }
    }

    impl HostBackend for RecordingHost {
        fn context(&self) -> Result<Context> {
            Err(PortakiError::HostNotConfigured)
        }
        fn kv_get(&self, _: &str) -> Result<Option<Vec<u8>>> {
            Ok(None)
        }
        fn kv_set(&self, _: &str, _: &[u8], _: Option<u32>) -> Result<()> {
            Ok(())
        }
        fn kv_delete(&self, _: &str) -> Result<()> {
            Ok(())
        }
        fn kv_list(&self, _: &str) -> Result<Vec<String>> {
            Ok(vec![])
        }
        fn i18n_translate(&self, key: &str, _: &str) -> Result<String> {
            Ok(key.into())
        }
        fn log(&self, _: &str, _: &str, _: &str) -> Result<()> {
            Ok(())
        }
        fn connector_call(&self, _: &str, _: &str, _: &str) -> Result<String> {
            Ok("{}".into())
        }
        fn emit_event(&self, _: &str, _: &str) -> Result<()> {
            Ok(())
        }
        fn email_send(&self, payload_json: &str) -> Result<()> {
            if let Some(code) = self.refuse_with {
                return Err(PortakiError::Host(format!("{code}: refused by host")));
            }
            self.sent.lock().unwrap().push(payload_json.to_string());
            Ok(())
        }
        fn time_now_iso(&self) -> Result<String> {
            Ok(self.now.to_rfc3339())
        }
    }

    fn instant(iso: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(iso)
            .unwrap()
            .with_timezone(&Utc)
    }

    fn args(audience: EmailAudience) -> SendEmailArgs {
        SendEmailArgs {
            email_id: "reminder".into(),
            audience,
            content: ModuleEmailSdui {
                subject: LocalizedEmailText::both("Subject"),
                body: LocalizedEmailText::both("Body"),
                ..Default::default()
            },
            stay_id: None,
            property_id: None,
            action_url: None,
        }
    }

    fn stay_ctx(stay_id: Uuid, checkout: &str) -> Context {
        Context {
            stay: Some(StayContext {
                stay_id,
                checkin_at: None,
                checkout_at: Some(instant(checkout)),
                ..StayContext::default()
            }),
            ..Context::default()
        }
    }

    #[test]
    fn caps_count_chars_not_bytes() {
        let mut ok = args(EmailAudience::Host);
        ok.content.subject = LocalizedEmailText::both("é".repeat(limits::EMAIL_SUBJECT_MAX_CHARS));
        assert_eq!(ok.validate(), Ok(()));

        let mut over = ok.clone();
        over.content
            .subject
            .translations
            .insert("de".into(), "ä".repeat(limits::EMAIL_SUBJECT_MAX_CHARS + 1));
        assert_eq!(
            over.validate(),
            Err(EmailError::FieldTooLong {
                field: EmailField::Subject,
                locale: "de".into(),
                actual: limits::EMAIL_SUBJECT_MAX_CHARS + 1,
                max: limits::EMAIL_SUBJECT_MAX_CHARS,
            })
        );
    }

    type SetCopy = fn(&mut SendEmailArgs, LocalizedEmailText);

    #[test]
    fn every_copy_field_has_its_cap() {
        let cases: [(EmailField, usize, SetCopy); 4] = [
            (
                EmailField::Eyebrow,
                limits::EMAIL_EYEBROW_MAX_CHARS,
                |a, t| a.content.eyebrow = Some(t),
            ),
            (EmailField::Title, limits::EMAIL_TITLE_MAX_CHARS, |a, t| {
                a.content.title = Some(t)
            }),
            (EmailField::Body, limits::EMAIL_BODY_MAX_CHARS, |a, t| {
                a.content.body = t
            }),
            (
                EmailField::CtaLabel,
                limits::EMAIL_CTA_LABEL_MAX_CHARS,
                |a, t| {
                    a.content.cta = Some(ModuleEmailCta {
                        label: t,
                        ..Default::default()
                    })
                },
            ),
        ];
        for (field, max, set) in cases {
            let mut at_cap = args(EmailAudience::Guest);
            set(&mut at_cap, LocalizedEmailText::both("x".repeat(max)));
            assert_eq!(at_cap.validate(), Ok(()), "{field} at cap");

            let mut over = args(EmailAudience::Guest);
            set(
                &mut over,
                LocalizedEmailText::new("x".repeat(max + 1), "ok"),
            );
            let err = over.validate().unwrap_err();
            assert_eq!(err.code(), "email_field_too_long");
            assert!(
                matches!(err, EmailError::FieldTooLong { field: f, .. } if f == field),
                "{field}: {err}"
            );
        }
    }

    #[test]
    fn blank_required_fields_are_refused() {
        let mut no_id = args(EmailAudience::Host);
        no_id.email_id = "  ".into();
        assert_eq!(
            no_id.validate(),
            Err(EmailError::EmptyField {
                field: EmailField::EmailId
            })
        );

        let mut no_subject = args(EmailAudience::Host);
        no_subject.content.subject = LocalizedEmailText::new(" ", "");
        assert_eq!(
            no_subject.validate(),
            Err(EmailError::EmptyField {
                field: EmailField::Subject
            })
        );

        // One non-blank locale is enough: the fallback chain resolves to it.
        let mut de_only = args(EmailAudience::Host);
        de_only.content.body = LocalizedEmailText::default();
        de_only
            .content
            .body
            .translations
            .insert("de".into(), "Hallo".into());
        assert_eq!(de_only.validate(), Ok(()));

        let mut no_body = args(EmailAudience::Host);
        no_body.content.body = LocalizedEmailText::default();
        assert_eq!(
            no_body.validate(),
            Err(EmailError::EmptyField {
                field: EmailField::Body
            })
        );
    }

    #[test]
    fn action_url_must_be_absolute_https() {
        for url in ["https://app.portaki.app/stays/1", "HTTPS://app.portaki.app"] {
            let mut ok = args(EmailAudience::Host);
            ok.action_url = Some(url.into());
            assert_eq!(ok.validate(), Ok(()), "{url}");
        }
        for url in [
            "http://app.portaki.app",
            "/stays/1",
            "https://",
            "https:///path",
            "javascript:alert(1)",
            "",
        ] {
            let mut bad = args(EmailAudience::Host);
            bad.action_url = Some(url.into());
            assert_eq!(bad.validate(), Err(EmailError::ActionUrlNotHttps), "{url}");
        }
    }

    #[test]
    fn after_stay_rule_only_judges_the_invocation_stay() {
        let stay = Uuid::from_u128(1);
        let ctx = stay_ctx(stay, "2026-06-08T10:00:00Z");
        let within = instant("2026-06-15T10:00:00Z");
        let past = instant("2026-06-15T10:00:01Z");

        let guest = args(EmailAudience::Guest);
        assert!(
            !guest.is_after_stay(&ctx, within),
            "exactly 7 days is still allowed"
        );
        assert!(guest.is_after_stay(&ctx, past));

        let mut same_stay = guest.clone();
        same_stay.stay_id = Some(stay);
        assert!(same_stay.is_after_stay(&ctx, past));

        let mut other_stay = guest.clone();
        other_stay.stay_id = Some(Uuid::from_u128(2));
        assert!(!other_stay.is_after_stay(&ctx, past), "unknown checkout");

        assert!(!args(EmailAudience::Host).is_after_stay(&ctx, past));
        assert!(!guest.is_after_stay(&Context::default(), past), "no stay");
    }

    #[test]
    fn send_refuses_an_ended_stay_without_calling_the_host() {
        let host = RecordingHost::at("2026-07-01T00:00:00Z");
        let ctx = stay_ctx(Uuid::from_u128(1), "2026-06-08T10:00:00Z");
        let result = with_host(host.clone(), ctx, || send(&args(EmailAudience::Guest)));
        assert!(matches!(
            result,
            Err(PortakiError::Email(EmailError::StayEnded))
        ));
        assert_eq!(host.sent(), 0);
    }

    #[test]
    fn send_refuses_invalid_content_without_calling_the_host() {
        let host = RecordingHost::at("2026-06-01T00:00:00Z");
        let mut bad = args(EmailAudience::Host);
        bad.action_url = Some("http://example.com".into());
        let result = with_host(host.clone(), Context::default(), || send(&bad));
        assert!(matches!(
            result,
            Err(PortakiError::Email(EmailError::ActionUrlNotHttps))
        ));
        assert_eq!(host.sent(), 0);
    }

    #[test]
    fn send_passes_a_valid_email_through() {
        let host = RecordingHost::at("2026-06-10T00:00:00Z");
        let ctx = stay_ctx(Uuid::from_u128(1), "2026-06-08T10:00:00Z");
        with_host(host.clone(), ctx, || send(&args(EmailAudience::Guest))).expect("send");
        assert_eq!(host.sent(), 1);
    }

    #[test]
    fn host_refusal_codes_come_back_typed() {
        for (code, expected) in [
            ("email_stay_ended", EmailError::StayEnded),
            ("email_limit_exceeded", EmailError::LimitExceeded),
        ] {
            let host = Arc::new(RecordingHost {
                now: instant("2026-06-10T00:00:00Z"),
                sent: Mutex::new(Vec::new()),
                refuse_with: Some(code),
            });
            let result = with_host(host, Context::default(), || {
                send(&args(EmailAudience::Host))
            });
            match result {
                Err(PortakiError::Email(err)) => {
                    assert_eq!(err, expected);
                    assert_eq!(err.code(), code);
                }
                other => panic!("{code}: {other:?}"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(text: &str) -> LocalizedEmailText {
        LocalizedEmailText::both(text)
    }

    fn with_blocks(hero: EmailHero, blocks: Vec<EmailBlock>) -> SendEmailArgs {
        SendEmailArgs {
            email_id: "digest".into(),
            audience: EmailAudience::Host,
            content: ModuleEmailSdui {
                subject: t("Subject"),
                body: t("Body"),
                hero,
                blocks,
                ..Default::default()
            },
            stay_id: None,
            property_id: None,
            action_url: None,
        }
    }

    #[test]
    fn legacy_content_serializes_without_hero_or_blocks() {
        let json = serde_json::to_value(with_blocks(EmailHero::Intro, vec![]).content).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"subject": {"fr": "Subject", "en": "Subject"}, "body": {"fr": "Body", "en": "Body"}})
        );
        let back: ModuleEmailSdui = serde_json::from_value(json).unwrap();
        assert_eq!(back.hero, EmailHero::Intro);
        assert!(back.blocks.is_empty());
    }

    #[test]
    fn blocks_and_hero_match_the_platform_wire() {
        let args = with_blocks(
            EmailHero::Alert {
                tone: EmailTone::Warning,
            },
            vec![
                EmailBlock::receipt(t("Reçu"), [EmailPair::new(t("Départ"), t("+25 €"))])
                    .with_meta(t("L’Islette"))
                    .with_total(t("Total"), t("25 €")),
                EmailBlock::stats([
                    EmailPair::new(t("Nouveaux"), t("4")),
                    EmailPair::new(t("Mis à jour"), t("2")),
                ])
                .ink(),
                EmailBlock::places([EmailPlace::new(t("Plage")).with_emoji("🏖️")])
                    .labeled(t("Autour")),
                EmailBlock::note(EmailTone::Info, t("À savoir")),
            ],
        );
        assert_eq!(args.validate(), Ok(()));
        let json = serde_json::to_value(&args.content).unwrap();
        assert_eq!(
            json["hero"],
            serde_json::json!({"kind": "alert", "tone": "warning"})
        );
        let blocks = &json["blocks"];
        assert_eq!(blocks[0]["type"], "receipt");
        assert_eq!(blocks[0]["totalLabel"]["fr"], "Total");
        assert_eq!(blocks[0]["items"][0]["value"]["en"], "+25 €");
        assert_eq!(blocks[1]["type"], "stats");
        assert_eq!(blocks[1]["ink"], true);
        assert_eq!(blocks[2]["items"][0]["emoji"], "🏖️");
        assert_eq!(blocks[2]["label"]["fr"], "Autour");
        assert_eq!(blocks[3]["tone"], "info");
        assert!(blocks[3].get("label").is_none());
        let back: ModuleEmailSdui = serde_json::from_value(json).unwrap();
        assert_eq!(back, args.content);
    }

    #[test]
    fn blocks_outside_the_contract_are_refused() {
        let pair = || EmailPair::new(t("a"), t("1"));
        let cases: Vec<(EmailBlock, &str)> = vec![
            (EmailBlock::checklist([]), "item_count"),
            (
                EmailBlock::checklist(vec![t("x"); limits::EMAIL_BLOCK_ITEMS_MAX + 1]),
                "item_count",
            ),
            (EmailBlock::stats([pair()]), "item_count"),
            (EmailBlock::stats([pair(), pair(), pair()]), "item_count"),
            (
                EmailBlock::rows([EmailPair::new(t("a"), t(" "))]),
                "required_text_blank",
            ),
            (
                EmailBlock::checklist([t(&"é".repeat(limits::EMAIL_BLOCK_TEXT_MAX_CHARS + 1))]),
                "text_too_long",
            ),
            (
                EmailBlock::note(EmailTone::Info, t("ok"))
                    .labeled(t(&"x".repeat(limits::EMAIL_BLOCK_TEXT_MAX_CHARS + 1))),
                "text_too_long",
            ),
            (
                EmailBlock::Receipt {
                    label: t("Reçu"),
                    meta: None,
                    items: vec![pair()],
                    total_label: None,
                    total: Some(t("25 €")),
                },
                "total_and_label_together",
            ),
        ];
        for emoji in ["", "a", "<", "🍝 ", "é", "🏖️🏖️🏖️🏖️🏖️🏖️"] {
            let place = EmailPlace::new(t("n")).with_emoji(emoji);
            assert!(!is_block_emoji(emoji), "{emoji:?}");
            let err = with_blocks(EmailHero::Intro, vec![EmailBlock::places([place])])
                .validate()
                .unwrap_err();
            assert_eq!(err.code(), "email_block_invalid");
        }
        for (block, reason) in cases {
            let err = with_blocks(
                EmailHero::Intro,
                vec![EmailBlock::checklist([t("ok")]), block],
            )
            .validate()
            .unwrap_err();
            assert!(
                matches!(err, EmailError::InvalidBlock { index: 1, reason: r, .. } if r == reason),
                "{reason}: {err}"
            );
        }
        let too_many = vec![EmailBlock::checklist([t("x")]); limits::EMAIL_BLOCKS_MAX + 1];
        assert_eq!(
            with_blocks(EmailHero::Ink, too_many).validate(),
            Err(EmailError::TooManyBlocks)
        );
        for emoji in ["🍝", "🏖️", "🇫🇷", "👩‍👩‍👧"] {
            assert!(is_block_emoji(emoji), "{emoji}");
        }
    }

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
        assert_eq!(
            text.translations.get("de").map(String::as_str),
            Some("Hallo Ada")
        );
        assert_eq!(text.resolve("de"), "Hallo Ada");
    }

    #[test]
    fn locale_fallback_chain_order() {
        assert_eq!(
            locale_fallback_chain("zh-CN"),
            vec![
                "zh-cn".to_string(),
                "zh".to_string(),
                "en".to_string(),
                "fr".to_string()
            ]
        );
        assert_eq!(
            locale_fallback_chain("en"),
            vec!["en".to_string(), "fr".to_string()]
        );
    }
}
