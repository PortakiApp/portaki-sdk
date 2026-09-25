//! `query` expansion — read-only gateway operations with Wasm JSON dispatch.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{ItemFn, Token};

use crate::emit::{sanitize_key, write_emission};
use crate::wire_lit::WireLit;

/// `name = "…"`, then an optional bare `guest` and any number of `example(…)` — shared by `query`
/// and `command`.
pub(crate) struct NamedOpAttrs {
    pub name: String,
    /// Guest-only: callable through the guest gateway, refused through the host one. Host-only unless the module says so.
    pub guest: bool,
    /// `example(label = "…", input = r#"{…}"#)` — what the sandbox's Exécuter tab offers to run.
    pub examples: Vec<serde_json::Value>,
}

/// `example(label = "…", input = "{…}")`, the input checked as a JSON object here, at compile
/// time: a typo would otherwise surface as a dead button in the sandbox.
fn parse_example(input: ParseStream<'_>) -> syn::Result<serde_json::Value> {
    let content;
    syn::parenthesized!(content in input);
    let mut label = None;
    let mut json = None;
    while !content.is_empty() {
        let key: syn::Ident = content.parse()?;
        content.parse::<Token![=]>()?;
        let value: syn::LitStr = content.parse()?;
        match key.to_string().as_str() {
            "label" if label.is_none() && !value.value().trim().is_empty() => {
                label = Some(value.value())
            }
            "input" if json.is_none() => {
                let parsed: serde_json::Value =
                    serde_json::from_str(&value.value()).map_err(|error| {
                        syn::Error::new(value.span(), format!("input is not JSON: {error}"))
                    })?;
                if !parsed.is_object() {
                    return Err(syn::Error::new(value.span(), "input must be a JSON object"));
                }
                json = Some(parsed);
            }
            _ => {
                return Err(syn::Error::new(
                    key.span(),
                    "expected label = \"…\" or input = \"{…}\", once each",
                ))
            }
        }
        content.parse::<Option<Token![,]>>()?;
    }
    let label = label.ok_or_else(|| content.error("example needs label = \"…\""))?;
    Ok(serde_json::json!({
        "label": label,
        "input": json.unwrap_or_else(|| serde_json::json!({})),
    }))
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
        let mut examples = Vec::new();
        while input.parse::<Option<Token![,]>>()?.is_some() && !input.is_empty() {
            let flag: syn::Ident = input.parse()?;
            match flag.to_string().as_str() {
                "guest" if !guest && !input.peek(Token![=]) => guest = true,
                "example" => examples.push(parse_example(input)?),
                _ => {
                    return Err(syn::Error::new(
                        flag.span(),
                        "expected `guest` or `example(…)`",
                    ))
                }
            }
        }
        if !input.is_empty() {
            return Err(input.error("expected `,`"));
        }
        Ok(NamedOpAttrs {
            name: name.value,
            guest,
            examples,
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
    if !attrs.examples.is_empty() {
        declaration["examples"] = attrs.examples.clone().into();
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

    #[test]
    fn examples_are_read_in_order_with_their_json_input() {
        let attrs = parse(
            r#"name = "getCurrent", example(label = "Paris", input = "{\"city\":\"Paris\"}"), guest, example(label = "Empty")"#,
        )
        .unwrap();
        assert!(attrs.guest);
        assert_eq!(
            attrs.examples,
            vec![
                serde_json::json!({ "label": "Paris", "input": { "city": "Paris" } }),
                serde_json::json!({ "label": "Empty", "input": {} }),
            ]
        );
    }

    #[test]
    fn an_example_input_must_be_a_json_object() {
        assert!(parse(r#"name = "x", example(label = "a", input = "{nope")"#).is_err());
        assert!(parse(r#"name = "x", example(label = "a", input = "[1]")"#).is_err());
        assert!(parse(r#"name = "x", example(input = "{}")"#).is_err());
        assert!(parse(r#"name = "x", example(label = "a", colour = "b")"#).is_err());
    }
}
