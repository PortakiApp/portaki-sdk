//! Generated SDUI primitive structs (see `sdui_primitives.json`).

#![allow(missing_docs)]
#![allow(non_snake_case)]

/// A primitive struct of the SDUI catalog, tied to its [`Component`] variant.
///
/// Implemented by `build.rs` for every primitive of `sdui_primitives.json`, so generic code
/// (tree walkers, test assertions) can name a primitive by type instead of by string.
pub trait SduiPrimitive: Clone + Into<Component> {
    /// Wire name of the primitive: the `type` field of its JSON.
    const TYPE_NAME: &'static str;

    /// Borrows the primitive out of `node` when `node` is this variant.
    fn from_component(node: &Component) -> Option<&Self>;
}

include!(concat!(env!("OUT_DIR"), "/generated_sdui.rs"));
