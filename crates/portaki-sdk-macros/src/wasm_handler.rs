//! Generates wasm handler shims and `inventory` registration.

use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{FnArg, ItemFn, ReturnType, Type};

/// Registers a query handler (manifest `name` + Rust `fn` symbol).
pub fn register_query(operation_name: &str, fn_name: &str, function_item: &ItemFn) -> TokenStream2 {
    register_handler(
        &[operation_name, fn_name],
        HandlerKind::Query,
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
        function_item,
    )
}

/// Registers a surface renderer (`render_fn` symbol only).
pub fn register_surface(render_fn: &str, function_item: &ItemFn) -> TokenStream2 {
    register_handler(&[render_fn], HandlerKind::Surface, function_item)
}

enum HandlerKind {
    Query,
    Command,
    Surface,
}

fn register_handler(names: &[&str], kind: HandlerKind, function_item: &ItemFn) -> TokenStream2 {
    let fn_ident = &function_item.sig.ident;
    let shim_ident = format_ident!("__portaki_shim_{}", fn_ident);
    let name_literals: Vec<_> = names.iter().map(|n| quote! { #n }).collect();

    let invoke = match kind {
        HandlerKind::Surface => {
            let surface_call = if returns_result(function_item) {
                quote! { #fn_ident(ctx)? }
            } else {
                quote! { #fn_ident(ctx) }
            };
            quote! {
                let surface = #surface_call;
                ::serde_json::to_value(surface)
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
