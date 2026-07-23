//! Typed wire identifiers for module / shell / platform boundaries.
//!
//! Prefer these newtypes over free `&str` / `String` literals when calling
//! across Wasm, SDUI actions, host ops, or peer modules. JSON wire format stays
//! a plain string (`serde` transparent / `as_str`).
//!
//! ## Ownership model
//!
//! | Type | Scope | Closed? |
//! |------|-------|---------|
//! | [`SurfaceId`] | Per-module catalog (plus shared [`convention`] names) | No — modules define their own |
//! | [`OperationName`] | Command / query names; shared peer contracts in [`crate::contracts`] | No — except SDK contracts |
//! | [`ModuleId`] | Catalog / install module id | No — runtime peers are dynamic |
//! | [`EventType`] | Platform + shell + module-emitted events | Partially — see [`crate::contracts`] |
//! | [`crate::CapabilityId`] | Platform plan / connector grants | Yes — closed enum |
//!
//! # Examples
//!
//! ```
//! use portaki_sdk::ids::{convention, SurfaceId};
//! use portaki_sdk::sdui::action::{Action, OverlayArgs, OverlayPresentation};
//!
//! const FORECAST: SurfaceId = SurfaceId::new("explore.forecast");
//!
//! let overlay = Action::open_overlay(
//!     OverlayPresentation::BottomSheet,
//!     FORECAST,
//!     OverlayArgs::new().icon("cloud-sun"),
//! );
//! let home = Action::navigate(convention::HOME_CARD, None);
//! assert!(matches!(overlay, Action::OpenOverlay { .. }));
//! assert!(matches!(home, Action::Navigate { .. }));
//! ```
//!
//! Builders reject bare `&str` / `String` — declare wire literals once via
//! [`define_surface_ids!`] / [`define_operation_names!`] / [`define_event_types!`]
//! (or `::new` / `::from_static` in tests) and pass typed consts at every use site.

use std::borrow::Borrow;
use std::fmt;
use std::ops::Deref;

use serde::{Deserialize, Serialize};

/// Manifest / SDUI surface identifier (wire: JSON string).
///
/// Define module-local catalogs with [`define_surface_ids!`] or
/// [`SurfaceId::new`]. Shared booklet conventions live under [`convention`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SurfaceId(pub &'static str);

impl SurfaceId {
    /// Creates a surface id from a static wire string.
    pub const fn new(id: &'static str) -> Self {
        Self(id)
    }

    /// Stable wire string.
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl AsRef<str> for SurfaceId {
    fn as_ref(&self) -> &str {
        self.0
    }
}

impl Deref for SurfaceId {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl fmt::Display for SurfaceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl From<SurfaceId> for String {
    fn from(id: SurfaceId) -> Self {
        id.0.to_string()
    }
}

impl PartialEq<str> for SurfaceId {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for SurfaceId {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

/// Command or query operation name (wire: JSON string).
///
/// Use module-local consts for own ops; use [`crate::contracts`] for
/// cross-module protocols (e.g. smart-lock commands).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OperationName(pub &'static str);

impl OperationName {
    /// Creates an operation name from a static wire string.
    pub const fn new(name: &'static str) -> Self {
        Self(name)
    }

    /// Stable wire string.
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl AsRef<str> for OperationName {
    fn as_ref(&self) -> &str {
        self.0
    }
}

impl Deref for OperationName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl fmt::Display for OperationName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl From<OperationName> for String {
    fn from(name: OperationName) -> Self {
        name.0.to_string()
    }
}

impl PartialEq<str> for OperationName {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for OperationName {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

/// Catalog / install module identifier (wire: JSON string).
///
/// Static module ids use [`ModuleId::from_static`]; peer discovery returns owned
/// ids via [`ModuleId::new`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ModuleId(String);

impl ModuleId {
    /// Creates a module id from any string-like value.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Creates a module id from a static wire string (zero allocation).
    pub fn from_static(id: &'static str) -> Self {
        Self(id.to_string())
    }

    /// Stable wire string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for ModuleId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Deref for ModuleId {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for ModuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<ModuleId> for String {
    fn from(id: ModuleId) -> Self {
        id.0
    }
}

impl From<&ModuleId> for String {
    fn from(id: &ModuleId) -> Self {
        id.0.clone()
    }
}

impl PartialEq<str> for ModuleId {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for ModuleId {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl Borrow<str> for ModuleId {
    fn borrow(&self) -> &str {
        &self.0
    }
}

/// Domain or shell event type name (wire: JSON string).
///
/// Platform / shell catalogs live under [`crate::contracts`]. Module-private
/// emit names stay in the module crate via [`define_event_types!`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EventType(pub &'static str);

impl EventType {
    /// Creates an event type from a static wire string.
    pub const fn new(event_type: &'static str) -> Self {
        Self(event_type)
    }

    /// Stable wire string.
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl AsRef<str> for EventType {
    fn as_ref(&self) -> &str {
        self.0
    }
}

impl Deref for EventType {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl fmt::Display for EventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl From<EventType> for String {
    fn from(event_type: EventType) -> Self {
        event_type.0.to_string()
    }
}

impl PartialEq<str> for EventType {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for EventType {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

/// Shared booklet / host surface id conventions used by first-party modules.
///
/// These are **naming conventions**, not a closed platform enum. Modules may
/// declare additional private surfaces; prefer local [`define_surface_ids!`] for
/// those. Use these consts when targeting the common booklet slots so typos
/// fail at compile time.
pub mod convention {
    use super::SurfaceId;

    /// Guest booklet home card surface.
    pub const HOME_CARD: SurfaceId = SurfaceId::new("home.card");
    /// Guest explore / fullscreen detail surface.
    pub const EXPLORE_DETAIL: SurfaceId = SurfaceId::new("explore.detail");
    /// Host dashboard primary settings surface.
    pub const HOST_MAIN: SurfaceId = SurfaceId::new("main");
}

/// Declares a module-local [`SurfaceId`] catalog.
///
/// # Examples
///
/// ```
/// portaki_sdk::define_surface_ids! {
///     HOME_CARD = "home.card",
///     EXPLORE_FORECAST = "explore.forecast",
/// }
///
/// assert_eq!(HOME_CARD.as_str(), "home.card");
/// ```
#[macro_export]
macro_rules! define_surface_ids {
    ($($name:ident = $value:literal),+ $(,)?) => {
        $(
            pub const $name: $crate::ids::SurfaceId = $crate::ids::SurfaceId::new($value);
        )+
    };
}

/// Declares a module-local [`OperationName`] catalog (commands / queries).
///
/// # Examples
///
/// ```
/// portaki_sdk::define_operation_names! {
///     UPDATE_CONFIG = "updateConfig",
///     GET_CONFIG = "getConfig",
/// }
///
/// assert_eq!(UPDATE_CONFIG.as_str(), "updateConfig");
/// ```
#[macro_export]
macro_rules! define_operation_names {
    ($($name:ident = $value:literal),+ $(,)?) => {
        $(
            pub const $name: $crate::ids::OperationName = $crate::ids::OperationName::new($value);
        )+
    };
}

/// Declares a module-local [`EventType`] catalog for `host::events::emit`.
///
/// # Examples
///
/// ```
/// portaki_sdk::define_event_types! {
///     PROGRESS_UPDATED = "checklist.progress-updated",
/// }
///
/// assert_eq!(PROGRESS_UPDATED.as_str(), "checklist.progress-updated");
/// ```
#[macro_export]
macro_rules! define_event_types {
    ($($name:ident = $value:literal),+ $(,)?) => {
        $(
            pub const $name: $crate::ids::EventType = $crate::ids::EventType::new($value);
        )+
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdui::action::{Action, OverlayArgs, OverlayPresentation};

    #[test]
    fn surface_id_serializes_as_string() {
        let id = SurfaceId::new("explore.forecast");
        assert_eq!(
            serde_json::to_value(id).unwrap(),
            serde_json::json!("explore.forecast")
        );
        assert_eq!(String::from(id), "explore.forecast");
    }

    #[test]
    fn action_builders_accept_typed_ids() {
        let action = Action::open_overlay(
            OverlayPresentation::BottomSheet,
            convention::EXPLORE_DETAIL,
            OverlayArgs::new(),
        );
        match action {
            Action::OpenOverlay { surface_render, .. } => {
                assert_eq!(surface_render, "explore.detail");
            }
            other => panic!("unexpected {other:?}"),
        }

        let peer = ModuleId::from_static("nuki");
        let cmd = peer.command_empty(crate::contracts::smart_lock::UNLOCK);
        match cmd {
            Action::Command {
                module_id, name, ..
            } => {
                assert_eq!(module_id, "nuki");
                assert_eq!(name, "unlock");
            }
            other => panic!("unexpected {other:?}"),
        }
    }
}
