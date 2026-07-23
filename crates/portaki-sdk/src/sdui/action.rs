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
//! | [`Action::Navigate`] | Routes to another module surface by id or shell path |
//! | [`Action::External`] | Opens `tel:`, `mailto:`, or `https:` via OS handler |
//! | [`Action::OpenOverlay`] | Second surface render with presentation hint |
//! | [`Action::Emit`] | Client-side shell event (no Wasm round-trip) |
//!
//! Boundary builders take typed ids ([`ModuleId`], [`OperationName`], [`SurfaceId`],
//! [`EventType`], [`NavigateTarget`]) — bare `&str` / `String` are rejected at the
//! type layer. Wire JSON remains plain strings.
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
//! use portaki_sdk::contracts::smart_lock;
//! use portaki_sdk::ids::{ModuleId, SurfaceId};
//! use portaki_sdk::sdui::action::{
//!     json_value, Action, EmptyArgs, NavigateTarget, OverlayArgs, OverlayPresentation,
//! };
//! use serde::Serialize;
//!
//! #[derive(Serialize)]
//! struct DetailParams {
//!     id: String,
//! }
//!
//! // Prefer catalog / contract consts — not OperationName::new at call sites.
//! let module = ModuleId::from_static("nuki");
//! let cmd = Action::command(&module, smart_lock::UNLOCK, EmptyArgs {});
//! let nav = Action::navigate(
//!     SurfaceId::new("detail"),
//!     Some(json_value(DetailParams {
//!         id: "abc".into(),
//!     })),
//! );
//! let path_nav = Action::navigate(NavigateTarget::path("appliances/tv-1"), None);
//! let overlay = Action::open_overlay(
//!     OverlayPresentation::BottomSheet,
//!     SurfaceId::new("explore.forecast"),
//!     OverlayArgs::new()
//!         .icon("cloud-sun")
//!         .title("i18n:nav.weather"),
//! );
//! assert!(matches!(cmd, Action::Command { .. }));
//! assert!(matches!(nav, Action::Navigate { .. }));
//! assert!(matches!(path_nav, Action::Navigate { .. }));
//! assert!(matches!(overlay, Action::OpenOverlay { .. }));
//! ```

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ids::{EventType, ModuleId, OperationName, SurfaceId};

/// Empty JSON object `{}` for form-backed commands with no static args.
///
/// Prefer this over `json!({})` when the shell merges form fields into the
/// command payload at submit time.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct EmptyArgs {}

/// Serializes a typed value to [`serde_json::Value`] for navigate/emit payloads.
///
/// Prefer typed structs + this helper over `json!` in modules.
pub fn json_value(value: impl Serialize) -> Value {
    serde_json::to_value(value).unwrap_or(Value::Null)
}

/// Overlay presentation style for [`Action::OpenOverlay`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum OverlayPresentation {
    /// Centered modal dialog.
    Modal,
    /// Bottom sheet (default for guest explore overlays).
    #[default]
    BottomSheet,
    /// Fullscreen takeover.
    Fullscreen,
}

/// Chrome args for [`Action::OpenOverlay`] — sheet title / icon shown by shells.
///
/// Wire keys are camelCase (`title`, `icon`). Closed set of fields used by modules
/// today; shells may ignore unknown keys on the wire if older payloads linger.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct OverlayArgs {
    /// Overlay chrome title (often an `i18n:` key).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Lucide (or shell) icon name for the overlay header.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

impl OverlayArgs {
    /// Empty chrome args (no title / icon).
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the overlay chrome title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets the overlay chrome icon name.
    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }
}

/// Typed destination for [`Action::navigate`].
///
/// Use [`NavigateTarget::Surface`] / [`From<SurfaceId>`] for manifest surfaces.
/// Use [`NavigateTarget::Path`] for dynamic shell routes (e.g. `appliances/{id}`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavigateTarget {
    /// Manifest surface id.
    Surface(SurfaceId),
    /// Opaque shell path (not a declared surface id).
    Path(String),
}

impl NavigateTarget {
    /// Builds a [`NavigateTarget::Surface`].
    pub const fn surface(id: SurfaceId) -> Self {
        Self::Surface(id)
    }

    /// Builds a [`NavigateTarget::Path`] from any string-like route.
    pub fn path(path: impl Into<String>) -> Self {
        Self::Path(path.into())
    }

    /// Wire string written into [`Action::Navigate::to`].
    pub fn as_wire(&self) -> &str {
        match self {
            Self::Surface(id) => id.as_str(),
            Self::Path(path) => path.as_str(),
        }
    }
}

impl From<SurfaceId> for NavigateTarget {
    fn from(id: SurfaceId) -> Self {
        Self::Surface(id)
    }
}

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
    /// In-shell navigation to another surface or path.
    Navigate {
        /// Destination surface id or shell path (wire string).
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
        /// Presentation style.
        presentation: OverlayPresentation,
        /// Wasm render symbol / surface id for the overlay surface.
        surface_render: String,
        /// Overlay chrome args (title / icon) forwarded to the shell.
        #[serde(skip_serializing_if = "Option::is_none")]
        args: Option<OverlayArgs>,
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
    /// Builds a [`Action::Command`] with typed JSON args (`impl Serialize`).
    ///
    /// `module_id` and `name` are typed — use [`ModuleId::from_static`] /
    /// [`crate::define_operation_names!`] / [`crate::contracts`] at call sites.
    /// Pass [`EmptyArgs`] for `{}`, or a module command DTO.
    pub fn command(module_id: &ModuleId, name: OperationName, args: impl Serialize) -> Self {
        Action::Command {
            module_id: module_id.as_str().to_string(),
            name: name.as_str().to_string(),
            args: Some(json_value(args)),
        }
    }

    /// Builds a [`Action::Navigate`] with optional route params.
    ///
    /// Accepts [`SurfaceId`] (via [`From`]) or [`NavigateTarget::Path`] for
    /// dynamic shell routes. Prefer `Some(json_value(TypedParams { .. }))`.
    pub fn navigate(to: impl Into<NavigateTarget>, params: Option<Value>) -> Self {
        let target = to.into();
        Action::Navigate {
            to: target.as_wire().to_string(),
            params,
        }
    }

    /// Builds a [`Action::External`] URL action.
    pub fn external(url: impl Into<String>) -> Self {
        Action::External { url: url.into() }
    }

    /// Builds a [`Action::OpenOverlay`] with typed presentation and chrome args.
    ///
    /// `surface_render` must be a [`SurfaceId`]. `args` accepts `None`,
    /// `Some(OverlayArgs)`, or a bare [`OverlayArgs`] (`From<T> for Option<T>`).
    pub fn open_overlay(
        presentation: OverlayPresentation,
        surface_render: SurfaceId,
        args: impl Into<Option<OverlayArgs>>,
    ) -> Self {
        Action::OpenOverlay {
            presentation,
            surface_render: surface_render.as_str().to_string(),
            args: args.into(),
        }
    }

    /// Builds a [`Action::Emit`] client-side event.
    ///
    /// `event` must be an [`EventType`] (module catalog or [`crate::contracts::shell`]).
    /// Prefer `Some(json_value(TypedPayload { .. }))` over `json!`.
    pub fn emit(event: EventType, payload: Option<Value>) -> Self {
        Action::Emit {
            event: event.as_str().to_string(),
            payload,
        }
    }

    /// Builds a [`Action::Copy`] clipboard action.
    pub fn copy(value: impl Into<String>, toast_key: Option<String>) -> Self {
        Action::Copy {
            value: value.into(),
            toast_key,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{ModuleId, OperationName, SurfaceId};
    use serde_json::json;

    #[test]
    fn overlay_args_serialize_camel_case() {
        let args = OverlayArgs::new()
            .icon("cloud-sun")
            .title("i18n:nav.weather");
        let value = serde_json::to_value(&args).expect("serialize");
        assert_eq!(
            value,
            json!({
                "icon": "cloud-sun",
                "title": "i18n:nav.weather",
            })
        );
    }

    #[test]
    fn open_overlay_requires_surface_id() {
        let action = Action::open_overlay(
            OverlayPresentation::BottomSheet,
            SurfaceId::new("explore.forecast"),
            OverlayArgs::new().icon("cloud-sun"),
        );
        match action {
            Action::OpenOverlay {
                surface_render,
                args,
                ..
            } => {
                assert_eq!(surface_render, "explore.forecast");
                assert_eq!(args.unwrap().icon.as_deref(), Some("cloud-sun"));
            }
            other => panic!("expected OpenOverlay, got {other:?}"),
        }
    }

    #[test]
    fn empty_args_serialize_as_object() {
        assert_eq!(json_value(EmptyArgs {}), json!({}));
    }

    #[test]
    fn command_accepts_typed_args() {
        #[derive(Serialize)]
        struct Args {
            item_id: String,
        }

        const COMPLETE_ITEM: OperationName = OperationName::new("completeItem");
        let module = ModuleId::from_static("checklist");
        let action = Action::command(
            &module,
            COMPLETE_ITEM,
            Args {
                item_id: "abc".into(),
            },
        );
        match action {
            Action::Command {
                module_id,
                name,
                args,
            } => {
                assert_eq!(module_id, "checklist");
                assert_eq!(name, "completeItem");
                assert_eq!(args.unwrap()["item_id"], "abc");
            }
            other => panic!("expected Command, got {other:?}"),
        }
    }

    #[test]
    fn navigate_accepts_surface_or_path() {
        let surface = Action::navigate(SurfaceId::new("explore.detail"), None);
        match surface {
            Action::Navigate { to, .. } => assert_eq!(to, "explore.detail"),
            other => panic!("unexpected {other:?}"),
        }

        let path = Action::navigate(NavigateTarget::path("appliances/tv-1"), None);
        match path {
            Action::Navigate { to, .. } => assert_eq!(to, "appliances/tv-1"),
            other => panic!("unexpected {other:?}"),
        }
    }
}
