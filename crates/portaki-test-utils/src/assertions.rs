//! Depth-first assertion helpers over rendered [`portaki_sdk::sdui::surface::Surface`] trees.
//!
//! [`SurfaceAssertions`] walks `Component` children and matches primitives via
//! the [`PrimitiveTag`] trait. Extend coverage by implementing `PrimitiveTag`
//! for additional [`portaki_sdk::sdui::primitives`] types in test modules.
//!
//! # Example
//!
//! ```
//! use portaki_sdk::sdui::primitives::{Card, Text};
//! use portaki_sdk::sdui::surface::Surface;
//! use portaki_test_utils::SurfaceAssertions;
//!
//! let surface = Surface::new(Card::new().child(Text::new()));
//! let assertions = SurfaceAssertions::new(&surface);
//!
//! assertions.assert_contains::<Card>();
//! assert_eq!(assertions.find_all::<Card>().len(), 1);
//! ```

use portaki_sdk::sdui::component::Component;
use portaki_sdk::sdui::primitives::Card;
use portaki_sdk::sdui::surface::Surface;

/// Borrowed wrapper for fluent SDUI tree queries.
#[derive(Debug, Clone)]
pub struct SurfaceAssertions<'a> {
    surface: &'a Surface,
}

impl<'a> SurfaceAssertions<'a> {
    /// Creates assertions over `surface.root` and its descendants.
    pub fn new(surface: &'a Surface) -> Self {
        Self { surface }
    }

    /// Returns `true` if any node matches primitive `T`.
    pub fn contains_primitive<T: PrimitiveTag>(&self) -> bool {
        self.find::<T>().is_some()
    }

    /// Returns the first depth-first match for primitive `T`, if any.
    pub fn find<T: PrimitiveTag>(&self) -> Option<T::Output> {
        let mut stack = vec![&self.surface.root];
        while let Some(node) = stack.pop() {
            if let Some(found) = T::match_node(node) {
                return Some(found);
            }
            stack.extend(child_nodes(node));
        }
        None
    }

    /// Returns every depth-first match for primitive `T`.
    pub fn find_all<T: PrimitiveTag>(&self) -> Vec<T::Output> {
        let mut found = Vec::new();
        let mut stack = vec![&self.surface.root];
        while let Some(node) = stack.pop() {
            if let Some(item) = T::match_node(node) {
                found.push(item);
            }
            stack.extend(child_nodes(node));
        }
        found
    }

    /// Panics with `T::NAME` when no matching primitive exists.
    pub fn assert_contains<T: PrimitiveTag>(&self) {
        assert!(
            self.contains_primitive::<T>(),
            "expected surface to contain {}",
            T::NAME
        );
    }
}

/// Associates an SDUI primitive type with its [`Component`] variant for tree matching.
///
/// Implement for each primitive you assert in tests. The crate provides
/// [`PrimitiveTag`] for [`Card`] out of the box.
pub trait PrimitiveTag {
    /// Human-readable primitive name used in panic messages.
    const NAME: &'static str;
    /// Concrete value extracted when `match_node` succeeds.
    type Output: Clone;
    /// Attempts to downcast `node` to this primitive.
    fn match_node(node: &Component) -> Option<Self::Output>;
}

impl PrimitiveTag for Card {
    const NAME: &'static str = "Card";
    type Output = Card;

    fn match_node(node: &Component) -> Option<Self::Output> {
        match node {
            Component::Card(card) => Some(card.clone()),
            _ => None,
        }
    }
}

fn child_nodes(node: &Component) -> Vec<&Component> {
    match node {
        Component::Stack(inner) => inner.children.iter().collect(),
        Component::Card(inner) => inner.children.iter().collect(),
        Component::Section(inner) => inner.children.iter().collect(),
        Component::Group(inner) => inner.children.iter().collect(),
        Component::Grid(inner) => inner.children.iter().collect(),
        Component::SurfacePrimitive(inner) => inner.children.iter().collect(),
        Component::Hero(inner) => inner.children.iter().collect(),
        Component::List(inner) => inner.children.iter().collect(),
        Component::Form(inner) => inner.children.iter().collect(),
        Component::Pressable(inner) => inner.children.iter().collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use portaki_sdk::sdui::primitives::{Card, Text};
    use portaki_sdk::sdui::surface::Surface;

    use super::SurfaceAssertions;

    #[test]
    fn find_card_in_tree() {
        let surface = Surface::new(Card::new().child(Text::new()).child(Card::new()));
        let assertions = SurfaceAssertions::new(&surface);
        let cards = assertions.find_all::<Card>();
        assert_eq!(cards.len(), 2);
    }
}
