//! `surface` expansion — host/guest SDUI renderers in the manifest and Wasm dispatch table.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{ItemFn, LitStr, Token};

use crate::emit::{sanitize_key, write_emission};

struct SurfaceAttrs {
    context: String,
    id: String,
    display_name_key: Option<String>,
}

impl Parse for SurfaceAttrs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let context = if input.peek(LitStr) {
            input.parse::<LitStr>()?.value()
        } else {
            let ident: syn::Ident = input.parse()?;
            ident.to_string()
        };
        input.parse::<Token![,]>()?;
        let id_key: syn::Ident = input.parse()?;
        if id_key != "id" {
            return Err(syn::Error::new(id_key.span(), "expected id = \"...\""));
        }
        input.parse::<Token![=]>()?;
        let id: LitStr = input.parse()?;

        let mut display_name_key = None;
        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            let key: syn::Ident = input.parse()?;
            if key == "display_name_key" {
                input.parse::<Token![=]>()?;
                display_name_key = Some(input.parse::<LitStr>()?.value());
            }
        }

        Ok(SurfaceAttrs {
            context,
            id: id.value(),
            display_name_key,
        })
    }
}

/// Expands `#[surface(host|guest, id = "…", …)]` on a render function.
pub fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    let function_item = syn::parse_macro_input!(item as ItemFn);
    let attrs = syn::parse_macro_input!(attr as SurfaceAttrs);
    let fn_name = function_item.sig.ident.to_string();

    let display_name_key = attrs.display_name_key.unwrap_or_default();
    let display_fragment = if display_name_key.is_empty() {
        String::new()
    } else {
        format!(
            ",\n  \"displayNameKey\": {}",
            serde_json::to_string(&display_name_key).unwrap()
        )
    };

    let json = format!(
        r#"{{
  "kind": "surface",
  "context": {},
  "id": {},
  "renderFn": {}{}
}}"#,
        serde_json::to_string(&attrs.context).unwrap(),
        serde_json::to_string(&attrs.id).unwrap(),
        serde_json::to_string(&fn_name).unwrap(),
        display_fragment,
    );

    let key = format!("{}_{}", attrs.context, attrs.id);
    let emission = write_emission("surface", &sanitize_key(&key), &json);
    let wasm_registration = crate::wasm_handler::register_surface(&fn_name, &function_item);
    let output: TokenStream2 = quote! {
        #emission
        #function_item
        #wasm_registration
    };

    output.into()
}
