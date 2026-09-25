//! Wasm dispatch shims and `inventory::submit!` registration for annotated handlers.
//!
//! Used by [`query`](crate::query), [`command`](crate::command), and [`surface`](crate::surface).
//! Each shim:
//!
//! 1. Accepts `Context` + `serde_json::Value` params from the Extism entrypoint.
//! 2. Deserializes typed args (queries/commands with a second parameter).
//! 3. Invokes the author's function and serializes the return value.
//! 4. Maps errors to `PortakiError::Host` with `wasm_params_invalid` / `wasm_handler_failed` prefixes.
//!
//! Two registrations, one per target:
//!
//! - `wasm32`: a `HandlerRegistration` the Extism entry points dispatch through — unchanged, so a
//!   module binary carries nothing more than before.
//! - every other target: a `HandlerDeclaration` that also says what the handler is (query, command,
//!   surface and its context). Native `cargo test` reads it: the conformance battery of
//!   `portaki-test-utils` enumerates what a module declares from there, without a hand-kept list.

use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{FnArg, ItemFn, ReturnType, Type};

/// The const naming a declared id next to its handler — `home.card` → `HOME_CARD`,
/// `statsSummary` → `STATS_SUMMARY` — so a module keeps no hand-written `ids.rs`.
///
/// `kind` is `SurfaceId` or `OperationName`.
pub fn declared_const(kind: &str, wire: &str, fn_ident: &syn::Ident) -> TokenStream2 {
    let name = format_ident!("{}", const_name(wire));
    let ty = format_ident!("{}", kind);
    let doc = format!("`{wire}`, declared by [`{fn_ident}`].");
    quote! {
        #[doc = #doc]
        #[allow(dead_code)]
        pub const #name: ::portaki_sdk::ids::#ty = ::portaki_sdk::ids::#ty::new(#wire);
    }
}

/// `SCREAMING_SNAKE_CASE` of a wire id: camelCase humps and any non-alphanumeric run become `_`.
pub(crate) fn const_name(wire: &str) -> String {
    let mut out = String::new();
    let mut previous: Option<char> = None;
    for ch in wire.chars() {
        let hump = ch.is_ascii_uppercase()
            && previous.is_some_and(|p| p.is_ascii_lowercase() || p.is_ascii_digit());
        if (!ch.is_ascii_alphanumeric() || hump) && !out.is_empty() && !out.ends_with('_') {
            out.push('_');
        }
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_uppercase());
        }
        previous = Some(ch);
    }
    let out = out.trim_end_matches('_').to_string();
    if out.starts_with(|c: char| c.is_ascii_digit()) {
        format!("_{out}")
    } else {
        out
    }
}

/// Registers a query handler (manifest `name` + Rust `fn` symbol).
pub fn register_query(operation_name: &str, fn_name: &str, function_item: &ItemFn) -> TokenStream2 {
    register_handler(
        &[operation_name, fn_name],
        HandlerKind::Query,
        Declared {
            name: operation_name,
            context: "",
        },
        function_item,
    )
}

/// Registers a command handler.
pub fn register_command(
    operation_name: &str,
    fn_name: &str,
    function_item: &ItemFn,
) -> TokenStream2 {
    register_handler(
        &[operation_name, fn_name],
        HandlerKind::Command,
        Declared {
            name: operation_name,
            context: "",
        },
        function_item,
    )
}

/// Registers a surface renderer (`render_fn` symbol only).
///
/// `gated`: the shim renders through `portaki_sdk::guest_shell::render` — readiness states before
/// the call, the error state on `Err`.
pub fn register_surface(
    context: &str,
    surface_id: &str,
    gated: bool,
    render_fn: &str,
    function_item: &ItemFn,
) -> TokenStream2 {
    register_handler(
        &[render_fn],
        HandlerKind::Surface { gated },
        Declared {
            name: surface_id,
            context,
        },
        function_item,
    )
}

/// What the attribute declared, carried into the native declaration.
struct Declared<'a> {
    /// Operation name, or surface id.
    name: &'a str,
    /// `guest` / `host` for a surface, empty otherwise.
    context: &'a str,
}

enum HandlerKind {
    Query,
    Command,
    Surface { gated: bool },
}

fn register_handler(
    names: &[&str],
    kind: HandlerKind,
    declared: Declared<'_>,
    function_item: &ItemFn,
) -> TokenStream2 {
    let fn_ident = &function_item.sig.ident;
    let fn_name = fn_ident.to_string();
    let declared_name = declared.name;
    let declared_context = declared.context;
    let kind_tokens = match kind {
        HandlerKind::Query => quote! { ::portaki_sdk::wasm::registry::HandlerKind::Query },
        HandlerKind::Command => quote! { ::portaki_sdk::wasm::registry::HandlerKind::Command },
        HandlerKind::Surface { .. } => {
            quote! { ::portaki_sdk::wasm::registry::HandlerKind::Surface }
        }
    };
    let shim_ident = format_ident!("__portaki_shim_{}", fn_ident);
    let name_literals: Vec<_> = names.iter().map(|n| quote! { #n }).collect();

    let invoke = match kind {
        HandlerKind::Surface { gated } => {
            let (ctx_ty, args_ty) = parse_query_command_sig(function_item);
            let read_args = args_ty.as_ref().map(|args_ty| {
                quote! {
                    let args: #args_ty = ::serde_json::from_value(params)
                        .map_err(|e| ::portaki_sdk::error::PortakiError::Host(format!("wasm_params_invalid: {e}")))?;
                }
            });
            let call = if args_ty.is_some() {
                quote! { #fn_ident(ctx, args) }
            } else {
                quote! { #fn_ident(ctx) }
            };
            let surface = match (gated, returns_result(function_item)) {
                (true, true) => {
                    let surface_id = declared.name;
                    quote! {
                        ::portaki_sdk::guest_shell::render(ctx, #surface_id, |ctx: #ctx_ty| {
                            #call.map_err(::core::convert::Into::into)
                        })
                    }
                }
                (true, false) => {
                    let surface_id = declared.name;
                    quote! {
                        ::portaki_sdk::guest_shell::render(ctx, #surface_id, |ctx: #ctx_ty| Ok(#call))
                    }
                }
                (false, true) => quote! { #call? },
                (false, false) => call,
            };
            let surface_id = declared.name;
            quote! {
                let ctx: #ctx_ty = ctx;
                #read_args
                let surface = #surface;
                ::serde_json::to_value(surface)
                    .map(|value| ::portaki_sdk::sdui::surface::stamp_declared_id(value, #surface_id))
            }
        }
        HandlerKind::Query | HandlerKind::Command => {
            let (ctx_ty, args_ty) = parse_query_command_sig(function_item);
            match args_ty {
                Some(args_ty) => quote! {
                    let ctx: #ctx_ty = ctx;
                    let args: #args_ty = ::serde_json::from_value(params)
                        .map_err(|e| ::portaki_sdk::error::PortakiError::Host(format!("wasm_params_invalid: {e}")))?;
                    let out = #fn_ident(ctx, args)?;
                    ::serde_json::to_value(out)
                },
                None => quote! {
                    let ctx: #ctx_ty = ctx;
                    let out = #fn_ident(ctx)?;
                    ::serde_json::to_value(out)
                },
            }
        }
    };

    quote! {
        fn #shim_ident(
            ctx: ::portaki_sdk::context::Context,
            params: ::serde_json::Value,
        ) -> ::portaki_sdk::error::Result<::serde_json::Value> {
            #invoke
                .map_err(|e| ::portaki_sdk::error::PortakiError::Host(format!("wasm_handler_failed: {e}")))
        }

        #[cfg(target_arch = "wasm32")]
        ::portaki_sdk::inventory::submit! {
            ::portaki_sdk::wasm::HandlerRegistration {
                operation_names: &[ #(#name_literals),* ],
                dispatch: #shim_ident,
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        ::portaki_sdk::inventory::submit! {
            ::portaki_sdk::wasm::registry::HandlerDeclaration {
                kind: #kind_tokens,
                name: #declared_name,
                context: #declared_context,
                fn_name: #fn_name,
                dispatch: #shim_ident,
            }
        }
    }
}

fn parse_query_command_sig(function_item: &ItemFn) -> (TokenStream2, Option<TokenStream2>) {
    let mut args = function_item.sig.inputs.iter();
    let first = args.next().expect("handler must have a context parameter");
    let ctx_ty = match first {
        FnArg::Typed(pat_type) => &pat_type.ty,
        FnArg::Receiver(_) => panic!("handler must not take self"),
    };
    let ctx_ty_tokens = quote! { #ctx_ty };

    let second = args.next();
    if let Some(FnArg::Typed(pat_type)) = second {
        let args_ty = &pat_type.ty;
        return (ctx_ty_tokens, Some(quote! { #args_ty }));
    }

    (ctx_ty_tokens, None)
}

fn returns_result(function_item: &ItemFn) -> bool {
    match &function_item.sig.output {
        ReturnType::Default => false,
        ReturnType::Type(_, ty) => matches!(ty.as_ref(), Type::Path(path) if {
            path.path.segments.last().map(|s| s.ident == "Result").unwrap_or(false)
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::const_name;

    #[test]
    fn wire_ids_become_screaming_snake_case() {
        for (wire, name) in [
            ("home.card", "HOME_CARD"),
            ("post-stay.card", "POST_STAY_CARD"),
            ("main", "MAIN"),
            ("statsSummary", "STATS_SUMMARY"),
            ("legacyConfigAdopted", "LEGACY_CONFIG_ADOPTED"),
            ("getV2Items", "GET_V2_ITEMS"),
            ("calendar-sync", "CALENDAR_SYNC"),
            ("3d.view", "_3D_VIEW"),
        ] {
            assert_eq!(const_name(wire), name, "{wire}");
        }
    }
}
