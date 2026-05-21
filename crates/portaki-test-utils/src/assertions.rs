//! SDUI tree assertion helpers.

use portaki_sdk::sdui::component::Component;
use portaki_sdk::sdui::primitives::Card;
use portaki_sdk::sdui::surface::Surface;

/// Fluent assertions over a rendered [`Surface`].
#[derive(Debug, Clone)]
pub struct SurfaceAssertions<'a> {
    surface: &'a Surface,
}

impl<'a> SurfaceAssertions<'a> {
    /// Wraps a surface for assertions.
    pub fn new(surface: &'a Surface) -> Self {
        Self { surface }
    }

    /// Returns `true` when any node matches primitive `T`.
    pub fn contains_primitive<T: PrimitiveTag>(&self) -> bool {
        self.find::<T>().is_some()
    }

    /// Finds the first component matching `T`.
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

    /// Finds all components matching `T`.
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

    /// Asserts the tree contains at least one `T` primitive.
    pub fn assert_contains<T: PrimitiveTag>(&self) {
        assert!(
            self.contains_primitive::<T>(),
            "expected surface to contain {}",
            T::NAME
        );
    }
}

/// Maps a primitive struct to its [`Component`] variant for traversal.
pub trait PrimitiveTag {
    /// Primitive type name for error messages.
    const NAME: &'static str;
    /// Concrete primitive type extracted from the tree.
    type Output: Clone;
    /// Attempts to match a component node.
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
