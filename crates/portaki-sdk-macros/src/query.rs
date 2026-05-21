use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{ItemFn, LitStr, Token};

use crate::emit::{sanitize_key, write_emission};

struct NamedOpAttrs {
    name: String,
}

impl Parse for NamedOpAttrs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let key: syn::Ident = input.parse()?;
        if key != "name" {
            return Err(syn::Error::new(key.span(), "expected name = \"...\""));
        }
        input.parse::<Token![=]>()?;
        let name: LitStr = input.parse()?;
        Ok(NamedOpAttrs { name: name.value() })
    }
}

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

    let emission = write_emission("query", &sanitize_key(&attrs.name), quote! { #json });
    let output: TokenStream2 = quote! {
        #emission
        #function_item
    };

    output.into()
}
