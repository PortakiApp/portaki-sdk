//! Server-Driven UI (SDUI) catalog for module-authored interfaces.
//!
//! Modules build a tree of [`Component`] values, wrap them in [`Surface`], and
//! return that tree from `#[surface(...)]` handlers. Shells (host dashboard, guest
//! booklet) deserialize the JSON payload and render native widgets — modules never
//! ship JSX, Flutter, or CSS.
//!
//! ## Contract
//!
//! - Primitives and enums serialize to the schema version pinned in
//!   [`crate::manifest::UiSchemaVersions`].
//! - User interactions attach [`Action`] values; shells route them back to the
//!   gateway (commands, navigation, external URLs).
//! - Styling uses semantic tokens ([`common::Tone`], [`common::Emphasis`], …) —
//!   shells map tokens to platform theme — do not embed hex colors in Wasm.
//!
//! ## What modules must not assume
//!
//! - Not every shell implements every primitive — stick to the documented catalog.
//! - `Action::Command` crosses the Wasm boundary again; keep args small and JSON-serializable.
//! - Overlay surfaces (`Action::OpenOverlay`) are rendered in a second invocation —
//!   do not rely on in-memory state from the parent surface.
//!
//! # Examples
//!
//! ```ignore
//! use portaki_sdk::prelude::*;
//! use portaki_sdk::sdui::primitives::{Button, Card, Stack, Text};
//! use portaki_sdk::sdui::action::Action;
//!
//! fn render(ctx: HostContext) -> Surface {
//!     let refresh = Action::command(&ctx.module_id, "refresh", serde_json::json!({}));
//!     Surface::new(
//!         Card::new()
//!             .child(Stack::new().child(Text::new()))
//!             .child(Button::new().action(refresh)),
//!     )
//! }
//! ```

pub mod action;
pub mod common;
pub mod component;
pub mod primitives;
pub mod surface;

pub use action::Action;
pub use common::{Animation, Emphasis, SurfaceLevel, Tone, Visibility};
pub use component::Component;
pub use surface::Surface;
