//! Connector macros — built-in catalog references, custom HTTP connectors, and operations.
//!
//! [`expand_builtin`] (`connector`), [`expand_custom`] (`custom_connector`), and [`expand_op`]
//! (`connector_op`). Custom connector ops are merged onto the last custom connector by
//! `portaki-cli` manifest generation order.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{ItemFn, ItemStruct, LitStr, Token};

use crate::emit::{sanitize_key, write_emission};

struct BuiltinConnectorAttrs {
    builtin: String,
}

impl Parse for BuiltinConnectorAttrs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let key: syn::Ident = input.parse()?;
        if key != "builtin" {
            return Err(syn::Error::new(key.span(), "expected builtin = \"...\""));
        }
        input.parse::<Token![=]>()?;
        let builtin: LitStr = input.parse()?;
        Ok(BuiltinConnectorAttrs {
            builtin: builtin.value(),
        })
    }
}

struct CustomConnectorAttrs {
    id: String,
    display_name_key: Option<String>,
    base_url: Option<String>,
    credential_provider_id: Option<String>,
    /// Optional egress auth. For a connector the module declares itself (ADR-0021), one of the
    /// platform's forms: `bearer` | `basic` | `header:<name>` | `query:<name>` | `none`. The
    /// historical `query_appid` / `query_key` stay accepted for catalogued providers.
    auth: Option<String>,
}

/// The auth forms the platform knows how to inject. Anything else would only fail at the first
/// call, in the host's booklet; refusing it here makes it a build error instead.
fn is_known_auth(auth: &str) -> bool {
    let name_ok = |name: &str, extra: &str| {
        !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || extra.contains(c))
    };
    match auth.split_once(':') {
        None => matches!(
            auth,
            "bearer" | "basic" | "none" | "query_appid" | "query_key"
        ),
        Some(("header", name)) => name_ok(name, ""),
        Some(("query", name)) => name_ok(name, "_."),
        Some(_) => false,
    }
}

impl Parse for CustomConnectorAttrs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut id = None;
        let mut display_name_key = None;
        let mut base_url = None;
        let mut credential_provider_id = None;
        let mut auth = None;

        while !input.is_empty() {
            let key: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            let value: LitStr = input.parse()?;
            let text = value.value();

            match key.to_string().as_str() {
                "id" => id = Some(text),
                "display_name_key" => display_name_key = Some(text),
                "base_url" => base_url = Some(text),
                "credential_provider_id" => credential_provider_id = Some(text),
                "auth" => {
                    if !is_known_auth(&text) {
                        return Err(syn::Error::new(
                            value.span(),
                            format!(
                                "unknown auth `{text}`: expected bearer, basic, none, header:<name> or query:<name>"
                            ),
                        ));
                    }
                    auth = Some(text)
                }
                other => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!("unknown #[custom_connector] attribute: {other}"),
                    ));
                }
            }

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(CustomConnectorAttrs {
            id: id.ok_or_else(|| syn::Error::new(input.span(), "id is required"))?,
            display_name_key,
            base_url,
            credential_provider_id,
            auth,
        })
    }
}

struct ConnectorOpAttrs {
    method: Option<String>,
    path: Option<String>,
    cache: Option<String>,
    validator: bool,
}

impl Parse for ConnectorOpAttrs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        if input.is_empty() {
            return Ok(ConnectorOpAttrs {
                method: None,
                path: None,
                cache: None,
                validator: true,
            });
        }

        let mut method = None;
        let mut path = None;
        let mut cache = None;

        while !input.is_empty() {
            let key: syn::Ident = input.parse()?;
            if key == "validator" && !input.peek(Token![=]) {
                return Ok(ConnectorOpAttrs {
                    method: None,
                    path: None,
                    cache: None,
                    validator: true,
                });
            }

            input.parse::<Token![=]>()?;
            let value: LitStr = input.parse()?;
            let text = value.value();

            match key.to_string().as_str() {
                "method" => method = Some(text),
                "path" => path = Some(text),
                "cache" => cache = Some(text),
                other => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!("unknown #[connector_op] attribute: {other}"),
                    ));
                }
            }

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(ConnectorOpAttrs {
            method,
            path,
            cache,
            validator: false,
        })
    }
}

/// Expands `#[connector(builtin = "…")]`.
pub fn expand_builtin(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = syn::parse_macro_input!(item as syn::Item);
    let attrs = syn::parse_macro_input!(attr as BuiltinConnectorAttrs);

    let json = format!(
        r#"{{
  "kind": "connector_builtin",
  "id": {}
}}"#,
        serde_json::to_string(&attrs.builtin).unwrap(),
    );

    let emission = write_emission("connector_builtin", &sanitize_key(&attrs.builtin), &json);
    let output: TokenStream2 = quote! {
        #emission
        #item
    };

    output.into()
}

/// Expands `#[custom_connector(…)]` on a marker struct.
pub fn expand_custom(attr: TokenStream, item: TokenStream) -> TokenStream {
    let struct_item = syn::parse_macro_input!(item as ItemStruct);
    let attrs = syn::parse_macro_input!(attr as CustomConnectorAttrs);

    let json = format!(
        r#"{{
  "kind": "connector_custom",
  "id": {},
  "displayNameKey": {},
  "baseUrl": {},
  "credentialProviderId": {},
  "auth": {}
}}"#,
        serde_json::to_string(&attrs.id).unwrap(),
        serde_json::to_string(&attrs.display_name_key).unwrap(),
        serde_json::to_string(&attrs.base_url).unwrap(),
        serde_json::to_string(&attrs.credential_provider_id).unwrap(),
        serde_json::to_string(&attrs.auth).unwrap(),
    );

    let emission = write_emission("connector_custom", &sanitize_key(&attrs.id), &json);
    let output: TokenStream2 = quote! {
        #emission
        #struct_item
    };

    output.into()
}

/// Expands `#[connector_op(…)]` on a function (HTTP op or `validator` stub).
pub fn expand_op(attr: TokenStream, item: TokenStream) -> TokenStream {
    let function_item = syn::parse_macro_input!(item as ItemFn);
    let attrs = syn::parse_macro_input!(attr as ConnectorOpAttrs);
    let fn_name = function_item.sig.ident.to_string();

    let json = format!(
        r#"{{
  "kind": "connector_op",
  "fn": {},
  "method": {},
  "path": {},
  "cache": {},
  "validator": {}
}}"#,
        serde_json::to_string(&fn_name).unwrap(),
        serde_json::to_string(&attrs.method).unwrap(),
        serde_json::to_string(&attrs.path).unwrap(),
        serde_json::to_string(&attrs.cache).unwrap(),
        attrs.validator,
    );

    let emission = write_emission("connector_op", &sanitize_key(&fn_name), &json);
    let output: TokenStream2 = quote! {
        #emission
        #function_item
    };

    output.into()
}

#[cfg(test)]
mod auth_form_tests {
    use super::is_known_auth;

    #[test]
    fn the_platform_forms_and_the_historical_styles_are_known() {
        for auth in [
            "bearer",
            "basic",
            "none",
            "header:exp-api-key",
            "query:api_key",
            "query_appid",
            "query_key",
        ] {
            assert!(is_known_auth(auth), "{auth}");
        }
    }

    #[test]
    fn anything_else_is_a_build_error() {
        for auth in [
            "Bearer",
            "digest",
            "header:",
            "header:X Bad",
            "query:",
            "cookie:sid",
            "header:a\r\nb",
        ] {
            assert!(!is_known_auth(auth), "{auth}");
        }
    }
}
