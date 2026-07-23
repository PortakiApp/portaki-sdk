//! `query` expansion — read-only gateway operations with Wasm JSON dispatch.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{ItemFn, Token};

use crate::emit::{sanitize_key, write_emission};
use crate::wire_lit::WireLit;

struct NamedOpAttrs {
    name: String,
}

impl Parse for NamedOpAttrs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let key: syn::Ident = input.parse()?;
        if key != "name" {
            return Err(syn::Error::new(
                key.span(),
                "expected name = \"...\" or name = OperationName::new(\"...\")",
            ));
        }
        input.parse::<Token![=]>()?;
        let name: WireLit = input.parse()?;
        Ok(NamedOpAttrs { name: name.value })
    }
}

/// Expands `#[query(name = "…")]` on a handler function.
pub fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    let function_item = syn::parse_macro_input!(item as ItemFn);
    let attrs = syn::parse_macro_input!(attr as NamedOpAttrs);
    let fn_name = function_item.sig.ident.to_string();

    let json = format!(
        r#"{{
  "kind": "query",
  "name": {},
  "fn": {}
}}"#,
        serde_json::to_string(&attrs.name).unwrap(),
        serde_json::to_string(&fn_name).unwrap(),
    );

    let emission = write_emission("query", &sanitize_key(&attrs.name), &json);
    let wasm_registration =
        crate::wasm_handler::register_query(&attrs.name, &fn_name, &function_item);
    let output: TokenStream2 = quote! {
        #emission
        #function_item
        #wasm_registration
    };

    output.into()
}
