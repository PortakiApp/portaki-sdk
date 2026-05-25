//! Proc-macros for Portaki Wasm modules.
//!
//! Each macro emits JSON metadata under `OUT_DIR/portaki-emissions/` for the CLI
//! to merge into `manifest.json` at build time.

mod capability;
mod command;
mod connector;
mod emit;
mod entity;
mod event_handler;
mod module;
mod query;
mod surface;
mod wasm_handler;

use proc_macro::TokenStream;

/// Crate-level inner attribute: `#![portaki_module(...)]` on `lib.rs`.
#[proc_macro_attribute]
pub fn portaki_module(attr: TokenStream, item: TokenStream) -> TokenStream {
    module::expand(attr, item)
}

/// Module declaration macro used at the root of `lib.rs` (`portaki_module!(...)`).
#[proc_macro]
pub fn portaki_module_decl(input: TokenStream) -> TokenStream {
    module::expand_invocation(input)
}

/// Declares a typed entity stored via `host::repo`.
#[proc_macro_attribute]
pub fn entity(attr: TokenStream, item: TokenStream) -> TokenStream {
    entity::expand_entity(attr, item)
}

/// Declares indexes for an entity.
#[proc_macro_attribute]
pub fn entity_indexes(attr: TokenStream, item: TokenStream) -> TokenStream {
    entity::expand_entity_indexes(attr, item)
}

/// Declares a host or guest surface renderer.
#[proc_macro_attribute]
pub fn surface(attr: TokenStream, item: TokenStream) -> TokenStream {
    surface::expand(attr, item)
}

/// Declares a query operation exposed to the gateway.
#[proc_macro_attribute]
pub fn query(attr: TokenStream, item: TokenStream) -> TokenStream {
    query::expand(attr, item)
}

/// Declares a command mutation exposed to the gateway.
#[proc_macro_attribute]
pub fn command(attr: TokenStream, item: TokenStream) -> TokenStream {
    command::expand(attr, item)
}

/// Declares an event subscription handler.
#[proc_macro_attribute]
pub fn event_handler(attr: TokenStream, item: TokenStream) -> TokenStream {
    event_handler::expand(attr, item)
}

/// Declares a required or optional capability dependency.
#[proc_macro_attribute]
pub fn capability(attr: TokenStream, item: TokenStream) -> TokenStream {
    capability::expand(attr, item)
}

/// Declares use of a built-in connector from the manifest.
#[proc_macro_attribute]
pub fn connector(attr: TokenStream, item: TokenStream) -> TokenStream {
    connector::expand_builtin(attr, item)
}

/// Declares a custom HTTP connector implemented by the module.
#[proc_macro_attribute]
pub fn custom_connector(attr: TokenStream, item: TokenStream) -> TokenStream {
    connector::expand_custom(attr, item)
}

/// Declares an operation on a custom connector (or validator stub).
#[proc_macro_attribute]
pub fn connector_op(attr: TokenStream, item: TokenStream) -> TokenStream {
    connector::expand_op(attr, item)
}
