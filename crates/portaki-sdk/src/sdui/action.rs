//! SDUI actions — declarative user interactions dispatched by shells.
//!
//! Attach [`Action`] values to interactive primitives (`Button`, `Pressable`, …).
//! When the guest or host activates the control, the shell sends the action to the
//! gateway — modules do not handle tap events inside Wasm directly.
//!
//! ## Contract
//!
//! | Variant | Gateway behaviour |
//! |---------|-------------------|
//! | [`Action::Command`] | Invokes a manifest [`crate::manifest::ManifestCommand`] |
//! | [`Action::Navigate`] | Routes to another module surface by id |
//! | [`Action::External`] | Opens `tel:`, `mailto:`, or `https:` via OS handler |
//! | [`Action::OpenOverlay`] | Second surface render with presentation hint |
//! | [`Action::Emit`] | Client-side shell event (no Wasm round-trip) |
//!
//! ## What modules must not assume
//!
//! - Commands may fail after validation — shells show generic error UI; return
//!   structured errors from command handlers when possible.
//! - `Navigate` params are shell-specific — keep keys stable and documented in manifest.
//! - `External` URLs are not sandboxed by the module — only emit trusted destinations.
//!
//! # Examples
//!
//! ```
//! use portaki_sdk::sdui::action::Action;
//! use serde_json::json;
//!
//! let cmd = Action::command("weather", "refresh", json!({ "force": true }));
//! let nav = Action::navigate("detail", Some(json!({ "id": "abc" })));
//! assert!(matches!(cmd, Action::Command { .. }));
//! ```

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Tagged action envelope serialized into SDUI interactive primitives.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Action {
    /// Invoke a module command through the gateway.
    Command {
        /// Target module id (typically `Context::module_id`).
        module_id: String,
        /// Command name from the manifest.
        name: String,
        /// JSON arguments object passed to the command handler.
        #[serde(skip_serializing_if = "Option::is_none")]
        args: Option<Value>,
    },
    /// In-shell navigation to another surface.
    Navigate {
        /// Destination surface id from the manifest.
        to: String,
        /// Optional route parameters forwarded to the target renderer.
        #[serde(skip_serializing_if = "Option::is_none")]
        params: Option<Value>,
    },
    /// Open an external URL in the system handler.
    External {
        /// Fully qualified URL (`tel:`, `mailto:`, `https:`).
        url: String,
    },
    /// Copy text to the device clipboard.
    Copy {
        /// Raw string copied by the shell.
        value: String,
        /// Optional i18n toast key shown after copy.
        #[serde(skip_serializing_if = "Option::is_none")]
        toast_key: Option<String>,
    },
    /// Trigger the native share sheet.
    Share {
        /// Share sheet title.
        title: String,
        /// Share body text.
        text: String,
        /// Canonical URL included in the share payload.
        url: String,
    },
    /// Compose an email or SMS message.
    ComposeMessage {
        /// Recipient address or phone.
        to: String,
        /// Message subject.
        subject: String,
        /// Message body.
        body: String,
    },
    /// Present a secondary surface rendered by the module.
    OpenOverlay {
        /// Presentation style: `modal`, `bottomSheet`, or `fullscreen`.
        presentation: String,
        /// Wasm render symbol for the overlay surface.
        surface_render: String,
        /// JSON args forwarded to the overlay renderer.
        #[serde(skip_serializing_if = "Option::is_none")]
        args: Option<Value>,
    },
    /// Emit a client-side event handled locally by the shell.
    Emit {
        /// Event name registered with the shell runtime.
        event: String,
        /// Optional JSON payload.
        #[serde(skip_serializing_if = "Option::is_none")]
        payload: Option<Value>,
    },
}

impl Action {
    /// Builds a [`Action::Command`] with the given JSON args.
    pub fn command(module_id: impl Into<String>, name: impl Into<String>, args: Value) -> Self {
        Action::Command {
            module_id: module_id.into(),
            name: name.into(),
            args: Some(args),
        }
    }

    /// Builds a [`Action::Navigate`] with optional route params.
    pub fn navigate(to: impl Into<String>, params: Option<Value>) -> Self {
        Action::Navigate {
            to: to.into(),
            params,
        }
    }
}
