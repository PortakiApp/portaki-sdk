//! `host::email` — ask the orchestrator to send a transactional email.
//!
//! Modules own the email **content** (subject / title / body SDUI payload). The
//! gateway wraps it in the guest (or host) Thymeleaf shell + property brand.
//!
//! Wire op: `email.send`. Runtime forwards the request as platform event
//! [`crate::contracts::platform::EMAIL_SEND`] so the orchestrator can resolve
//! recipients and render.

use serde::{Deserialize, Serialize};
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

/// Localized FR/EN string used in module email payloads.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct LocalizedEmailText {
    /// French copy.
    #[serde(default)]
    pub fr: String,
    /// English copy.
    #[serde(default)]
    pub en: String,
}

impl LocalizedEmailText {
    /// Builds from both locales.
    pub fn new(fr: impl Into<String>, en: impl Into<String>) -> Self {
        Self {
            fr: fr.into(),
            en: en.into(),
        }
    }

    /// Same string for FR and EN.
    pub fn both(text: impl Into<String>) -> Self {
        let text = text.into();
        Self {
            fr: text.clone(),
            en: text,
        }
    }
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
