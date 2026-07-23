//! `event_handler` expansion — platform event subscriptions (manifest only, no Wasm inventory).

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{ItemFn, Token};

use crate::emit::{sanitize_key, write_emission};
use crate::wire_lit::WireLit;

struct EventHandlerAttrs {
    event_type: String,
}

impl Parse for EventHandlerAttrs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let key: syn::Ident = input.parse()?;
        if key != "type" && key != "event_type" {
            return Err(syn::Error::new(
                key.span(),
                "expected type = \"...\" / event_type = \"...\" or EventType::new(\"...\")",
            ));
        }
        input.parse::<Token![=]>()?;
        let event_type: WireLit = input.parse()?;
        Ok(EventHandlerAttrs {
            event_type: event_type.value,
        })
    }
}

/// Expands `#[event_handler(event_type = "…")]` on a handler function.
pub fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    let function_item = syn::parse_macro_input!(item as ItemFn);
    let attrs = syn::parse_macro_input!(attr as EventHandlerAttrs);
    let fn_name = function_item.sig.ident.to_string();

    let json = format!(
        r#"{{
  "kind": "event_handler",
  "type": {},
  "handler": {}
}}"#,
        serde_json::to_string(&attrs.event_type).unwrap(),
        serde_json::to_string(&fn_name).unwrap(),
    );

    let emission = write_emission("event_handler", &sanitize_key(&attrs.event_type), &json);
    let output: TokenStream2 = quote! {
        #emission
        #function_item
    };

    output.into()
}
