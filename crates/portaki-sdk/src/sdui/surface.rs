//! Root SDUI document returned by `#[surface(...)]` render handlers.
//!
//! A [`Surface`] pairs an optional manifest surface id with a single [`super::component::Component`]
//! tree. The gateway forwards the serialized JSON to the requesting shell without
//! transformation beyond schema validation.
//!
//! ## Contract
//!
//! - `id` is usually omitted in production — the gateway injects the manifest id.
//!   Set it in tests via [`Surface::with_id`] for snapshot assertions.
//! - The tree must be acyclic; shells reject recursive component graphs.
//! - Render handlers should be pure given `Context` + host reads — no hidden globals.
//!
//! # Examples
//!
//! ```
//! use portaki_sdk::sdui::surface::Surface;
//! use portaki_sdk::sdui::primitives::{Stack, Text};
//!
//! let surface = Surface::new(Stack::new().child(Text::new()))
//!     .with_id("main");
//! assert_eq!(surface.id.as_deref(), Some("main"));
//! ```

use serde::{Deserialize, Serialize};

use super::component::Component;

/// Root SDUI tree for one module surface invocation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Surface {
    /// Manifest surface id — omitted in production responses unless explicitly set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Component tree rendered by the shell.
    pub root: Component,
}

impl Surface {
    /// Wraps a single root component (anything implementing `Into<Component>`).
    pub fn new(root: impl Into<Component>) -> Self {
        Self {
            id: None,
            root: root.into(),
        }
    }

    /// Sets the surface id — primarily for tests and navigation debugging.
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }
}
