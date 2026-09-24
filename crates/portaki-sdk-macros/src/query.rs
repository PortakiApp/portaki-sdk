//! `query` expansion — read-only gateway operations with Wasm JSON dispatch.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{ItemFn, Token};

use crate::emit::{sanitize_key, write_emission};
use crate::wire_lit::WireLit;

/// `name = "…"`, then an optional bare `guest` — shared by `query` and `command`.
pub(crate) struct NamedOpAttrs {
    pub name: String,
    /// Guest-only: callable through the guest gateway, refused through the host one. Host-only unless the module says so.
    pub guest: bool,
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
        let mut guest = false;
        if input.parse::<Option<Token![,]>>()?.is_some() && !input.is_empty() {
            let flag: syn::Ident = input.parse()?;
            if flag != "guest" {
                return Err(syn::Error::new(flag.span(), "expected `guest`"));
            }
            guest = true;
            input.parse::<Option<Token![,]>>()?;
        }
        Ok(NamedOpAttrs {
            name: name.value,
            guest,
        })
    }
}

/// Expands `#[query(name = "…")]` on a handler function.
pub fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    let function_item = syn::parse_macro_input!(item as ItemFn);
    let attrs = syn::parse_macro_input!(attr as NamedOpAttrs);
    let fn_name = function_item.sig.ident.to_string();

    let mut declaration = serde_json::json!({
        "kind": "query",
        "name": attrs.name,
        "fn": fn_name,
        "guest": attrs.guest,
    });
    // Le type d'arguments, par son nom : `portaki build` y joint les champs émis par son
    // `#[params]`, et la sandbox en tire un formulaire.
    if let Some(args) = crate::params::args_type_name(&function_item) {
        declaration["args"] = serde_json::Value::String(args);
    }
    let json = serde_json::to_string_pretty(&declaration).unwrap();

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

#[cfg(test)]
mod tests {
    use super::NamedOpAttrs;

    fn parse(attr: &str) -> syn::Result<NamedOpAttrs> {
        syn::parse_str(attr)
    }

    #[test]
    fn an_operation_is_closed_to_guests_by_default() {
        let attrs = parse(r#"name = "listForStay""#).unwrap();
        assert_eq!(attrs.name, "listForStay");
        assert!(!attrs.guest);
    }

    #[test]
    fn the_guest_flag_opens_it() {
        assert!(parse(r#"name = "submit", guest"#).unwrap().guest);
        assert!(parse(r#"name = "submit", guest,"#).unwrap().guest);
    }

    #[test]
    fn guest_takes_no_value_and_nothing_else_is_accepted() {
        assert!(parse(r#"name = "submit", guest = true"#).is_err());
        assert!(parse(r#"name = "submit", host"#).is_err());
        assert!(parse(r#"name = "submit", guest, guest"#).is_err());
    }
}
