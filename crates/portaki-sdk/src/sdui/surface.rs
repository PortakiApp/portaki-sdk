//! Surface root type returned by module renderers.

use serde::{Deserialize, Serialize};

use super::component::Component;

/// Root SDUI tree for a module surface.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Surface {
    /// Surface identifier from the manifest.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Root component tree.
    pub root: Component,
}

impl Surface {
    /// Wraps a single root component.
    pub fn new(root: impl Into<Component>) -> Self {
        Self {
            id: None,
            root: root.into(),
        }
    }

    /// Sets the surface id for tests and navigation.
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }
}
