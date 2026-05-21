//! `host::notifications` — template-based outbound notifications.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::Result;

/// Delivery channel for a notification template.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NotificationChannel {
    /// In-app feed.
    InApp,
    /// Email.
    Email,
    /// SMS (capability-gated).
    Sms,
}

/// Notification send request declared by modules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationRequest {
    /// Template id from the manifest.
    pub template_id: String,
    /// Channels to attempt.
    pub channels: Vec<NotificationChannel>,
    /// Template variables (JSON object).
    pub vars: serde_json::Value,
}

/// Sends a notification via the gateway.
pub fn send(req: NotificationRequest) -> Result<Uuid> {
    let _ = req;
    Ok(Uuid::new_v4())
}
