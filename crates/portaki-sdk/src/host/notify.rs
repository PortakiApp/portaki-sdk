//! `host::notify` — raise a HOST in-app inbox notification (bell) + push.
//!
//! Sibling of [`crate::host::email`]. Where `email::send` reaches the host over
//! transactional email, `notify::notify_host` raises an entry in the host
//! dashboard inbox and fans out a mobile push subject to the host's
//! [`NotificationCategory`] preference.
//!
//! Wire op: `host.notify`. The runtime forwards the request as platform event
//! [`crate::contracts::platform::HOST_NOTIFY`] so the orchestrator can resolve
//! the property host recipient(s), persist the inbox notification and trigger push.
//!
//! ## Localized copy
//!
//! Title / body reuse [`crate::host::email::LocalizedEmailText`] — the same
//! `{ "fr", "en", "translations" }` wire shape and fallback chain as module email.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::Result;
use crate::host::runtime::backend;

/// Localized copy shared with module email payloads.
pub use crate::host::email::LocalizedEmailText as LocalizedText;

/// Host push-preference category the notification is filed under.
///
/// Mirrors the Java `PushPreferenceCategory` enum — the host may mute a whole
/// category, in which case the inbox entry is still created but no push is sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NotificationCategory {
    /// Guest-originated messages / reports (issue reports, lost & found, …).
    GuestMessages,
    /// Configuration / setup alerts for the host.
    ConfigAlerts,
    /// Product news and announcements.
    ProductNews,
}

/// Arguments for [`notify_host`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NotifyHostArgs {
    /// Stable id for logs / delivery dedup (module-scoped), e.g. `submitted-{report_id}`.
    pub notification_id: String,
    /// Inbox title (bell + push title).
    pub title: LocalizedText,
    /// Inbox body (bell + push body).
    pub body: LocalizedText,
    /// Push-preference category the host can mute.
    pub category: NotificationCategory,
    /// Target stay scope (guest-originated notifications).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stay_id: Option<Uuid>,
    /// Property scope — used to resolve host recipient(s) + deep link.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub property_id: Option<Uuid>,
    /// Host dashboard deep-link.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_url: Option<String>,
}

/// Asks the orchestrator to raise `args` as a host inbox notification + push.
pub fn notify_host(args: &NotifyHostArgs) -> Result<()> {
    let payload_json = serde_json::to_string(args)?;
    backend()?.notify_host(&payload_json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_wire_format_is_camel_case() {
        assert_eq!(
            serde_json::to_value(NotificationCategory::GuestMessages).unwrap(),
            "guestMessages"
        );
        assert_eq!(
            serde_json::to_value(NotificationCategory::ConfigAlerts).unwrap(),
            "configAlerts"
        );
        assert_eq!(
            serde_json::to_value(NotificationCategory::ProductNews).unwrap(),
            "productNews"
        );
    }

    #[test]
    fn args_roundtrip_and_skip_optionals() {
        let args = NotifyHostArgs {
            notification_id: "submitted-42".into(),
            title: LocalizedText::new("Titre", "Title"),
            body: LocalizedText::new("Corps", "Body"),
            category: NotificationCategory::GuestMessages,
            stay_id: None,
            property_id: None,
            action_url: None,
        };
        let json = serde_json::to_string(&args).unwrap();
        assert!(json.contains("\"notificationId\":\"submitted-42\""));
        assert!(json.contains("\"category\":\"guestMessages\""));
        assert!(!json.contains("stayId"));
        assert!(!json.contains("propertyId"));
        assert!(!json.contains("actionUrl"));
        let back: NotifyHostArgs = serde_json::from_str(&json).unwrap();
        assert_eq!(back, args);
    }
}
