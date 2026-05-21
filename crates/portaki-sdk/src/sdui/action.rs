//! SDUI actions attached to interactive primitives.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// User interaction dispatched by shells back to the gateway.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Action {
    /// Invoke a module command.
    Command {
        /// Target module id (usually current module).
        module_id: String,
        /// Command name from the manifest.
        name: String,
        /// Command arguments object.
        #[serde(skip_serializing_if = "Option::is_none")]
        args: Option<Value>,
    },
    /// Navigate to another surface.
    Navigate {
        /// Surface id.
        to: String,
        /// Route parameters.
        #[serde(skip_serializing_if = "Option::is_none")]
        params: Option<Value>,
    },
    /// Open an external URL (`tel:`, `mailto:`, `https:`).
    External {
        /// Destination URL.
        url: String,
    },
    /// Copy a value to clipboard.
    Copy {
        /// Value to copy.
        value: String,
        /// i18n toast key after copy.
        #[serde(skip_serializing_if = "Option::is_none")]
        toast_key: Option<String>,
    },
    /// Native share sheet.
    Share {
        /// Share title.
        title: String,
        /// Share body.
        text: String,
        /// Share URL.
        url: String,
    },
    /// Compose an email/message.
    ComposeMessage {
        /// Recipient.
        to: String,
        /// Subject line.
        subject: String,
        /// Message body.
        body: String,
    },
    /// Open an overlay surface rendered by the module.
    OpenOverlay {
        /// `modal`, `bottomSheet`, or `fullscreen`.
        presentation: String,
        /// Surface render function name.
        surface_render: String,
        /// Arguments for the overlay surface.
        #[serde(skip_serializing_if = "Option::is_none")]
        args: Option<Value>,
    },
    /// Emit a client-side event handled by the shell.
    Emit {
        /// Event name.
        event: String,
        /// Event payload.
        #[serde(skip_serializing_if = "Option::is_none")]
        payload: Option<Value>,
    },
}

impl Action {
    /// Builds a command action.
    pub fn command(module_id: impl Into<String>, name: impl Into<String>, args: Value) -> Self {
        Action::Command {
            module_id: module_id.into(),
            name: name.into(),
            args: Some(args),
        }
    }

    /// Builds a navigate action.
    pub fn navigate(to: impl Into<String>, params: Option<Value>) -> Self {
        Action::Navigate {
            to: to.into(),
            params,
        }
    }
}
