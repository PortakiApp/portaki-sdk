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
//! use portaki_sdk::ids::convention;
//!
//! let surface = Surface::new(Stack::new().child(Text::new()))
//!     .with_id(convention::HOST_MAIN);
//! assert_eq!(surface.id.as_deref(), Some("main"));
//! ```

use serde::{Deserialize, Serialize};

use crate::ids::SurfaceId;

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

/// Sets `id` on a rendered surface that has none — what the `#[surface]` shim does with the
/// declared id, so a renderer need not call [`Surface::with_id`]. An id already set is kept.
#[doc(hidden)]
pub fn stamp_declared_id(mut surface: serde_json::Value, id: &str) -> serde_json::Value {
    if let serde_json::Value::Object(fields) = &mut surface {
        fields.entry("id").or_insert_with(|| id.into());
    }
    surface
}

impl Surface {
    /// Wraps a single root component (anything implementing `Into<Component>`).
    pub fn new(root: impl Into<Component>) -> Self {
        Self {
            id: None,
            root: root.into(),
        }
    }

    /// Sets the surface id. Rarely needed: the `#[surface]` dispatcher stamps the declared id on
    /// a surface that has none.
    ///
    /// Requires a [`SurfaceId`] (module catalog or [`crate::ids::convention`]).
    pub fn with_id(mut self, id: SurfaceId) -> Self {
        self.id = Some(id.as_str().to_string());
        self
    }
}

#[cfg(test)]
mod stamp_tests {
    use super::stamp_declared_id;
    use serde_json::json;

    #[test]
    fn the_declared_id_fills_a_missing_one_only() {
        assert_eq!(
            stamp_declared_id(json!({ "root": {} }), "main"),
            json!({ "id": "main", "root": {} })
        );
        assert_eq!(
            stamp_declared_id(json!({ "id": "other", "root": {} }), "main")["id"],
            "other"
        );
    }
}
