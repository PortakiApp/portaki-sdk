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
    /// platform's forms: `bearer` | `basic` | `header:<name>` | `none`. The historical
    /// `query_appid` / `query_key` stay accepted for catalogued providers.
    auth: Option<String>,
    /// ADR-0021: calls made with the publisher's key, per workspace and month. `None`: no cap
    /// (still counted).
    monthly_quota: Option<u64>,
    /// ADR-0021: where `oauth2_client_credentials` trades the key for an access token.
    token_url: Option<String>,
    /// Scopes asked at that exchange, space-separated.
    scopes: Option<String>,
    /// ADR-0021: fixed headers (`header = "Accept: application/json;version=2.0"`, repeatable).
    headers: std::collections::BTreeMap<String, String>,
    /// Headers taken from an argument (`header_arg = "Accept-Language=lang"`, repeatable).
    header_args: std::collections::BTreeMap<String, String>,
    /// Put before the key in a `header:<name>` form (`auth_prefix = "Token "`).
    auth_prefix: Option<String>,
    /// `false`: the publisher's key only, a host can never set theirs (a provider licence, say).
    host_key: bool,
}

/// What a module may never set itself: the transport.
const TRANSPORT_HEADERS: [&str; 8] = [
    "host",
    "content-length",
    "transfer-encoding",
    "connection",
    "keep-alive",
    "upgrade",
    "te",
    "trailer",
];

fn is_settable_header(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        && !TRANSPORT_HEADERS.contains(&name.to_ascii_lowercase().as_str())
}

fn is_visible_ascii(value: &str) -> bool {
    !value.is_empty() && value.len() <= 256 && value.chars().all(|c| (' '..='~').contains(&c))
}

/// A token URL receives the key: https, a host, and nothing that could hide another destination
/// (credentials, port, query, fragment). A path is allowed, unlike `base_url`.
fn is_valid_token_url(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("https://") else {
        return false;
    };
    let authority = rest.split('/').next().unwrap_or("");
    !authority.is_empty()
        && !authority.contains(['@', ':'])
        && !url.contains(['?', '#'])
        && !url.chars().any(char::is_whitespace)
}

/// The auth forms the platform knows how to inject. Anything else would only fail at the first
/// call, in the host's booklet; refusing it here makes it a build error instead.
fn is_known_auth(auth: &str) -> bool {
    match auth.split_once(':') {
        None => matches!(
            auth,
            "bearer" | "basic" | "none" | "query_appid" | "query_key" | "oauth2_client_credentials"
        ),
        // Not a transport or negotiation header: the key would break the request, or travel as
        // `Host`. The runtime refuses the same list.
        Some(("header", name)) => {
            !name.is_empty()
                && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
                && !RESERVED_HEADERS.contains(&name.to_ascii_lowercase().as_str())
        }
        // `query:<name>` waits until URL masking covers a publisher's parameter name.
        Some(_) => false,
    }
}

const RESERVED_HEADERS: [&str; 12] = [
    "host",
    "content-length",
    "transfer-encoding",
    "connection",
    "keep-alive",
    "upgrade",
    "te",
    "trailer",
    "accept",
    "content-type",
    "accept-encoding",
    "content-encoding",
];

/// The orchestrator files a publisher's key under `module:<module>/<connector>` and refuses any
/// other connector id; an uppercase id would compile, then fail as a missing key.
fn is_valid_connector_id(id: &str) -> bool {
    let mut chars = id.chars();
    id.len() <= 64
        && chars
            .next()
            .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

impl Parse for CustomConnectorAttrs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut id = None;
        let mut display_name_key = None;
        let mut base_url = None;
        let mut credential_provider_id = None;
        let mut auth = None;
        let mut monthly_quota = None;
        let mut token_url = None;
        let mut scopes = None;
        let mut headers = std::collections::BTreeMap::new();
        let mut header_args = std::collections::BTreeMap::new();
        let mut auth_prefix = None;
        let mut host_key = true;

        while !input.is_empty() {
            let key: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            // The one integer attribute: a cap on calls made with the publisher's key.
            if key == "host_key" {
                host_key = input.parse::<syn::LitBool>()?.value;
                if input.peek(Token![,]) {
                    input.parse::<Token![,]>()?;
                }
                continue;
            }
            if key == "monthly_quota" {
                let value: syn::LitInt = input.parse()?;
                let quota: u64 = value.base10_parse()?;
                if quota == 0 {
                    return Err(syn::Error::new(
                        value.span(),
                        "monthly_quota must be at least 1; leave it out for no cap",
                    ));
                }
                monthly_quota = Some(quota);
                if input.peek(Token![,]) {
                    input.parse::<Token![,]>()?;
                }
                continue;
            }
            let value: LitStr = input.parse()?;
            let text = value.value();

            match key.to_string().as_str() {
                "id" => {
                    if !is_valid_connector_id(&text) {
                        return Err(syn::Error::new(
                            value.span(),
                            format!(
                                "connector id `{text}`: lowercase letters, digits, `-` and `_`, 64 at most"
                            ),
                        ));
                    }
                    id = Some(text)
                }
                "display_name_key" => display_name_key = Some(text),
                "base_url" => base_url = Some(text),
                "credential_provider_id" => credential_provider_id = Some(text),
                "token_url" => {
                    if !is_valid_token_url(&text) {
                        return Err(syn::Error::new(
                            value.span(),
                            "token_url: an https URL with a host, no credentials, port, query or fragment",
                        ));
                    }
                    token_url = Some(text)
                }
                "scopes" => scopes = Some(text),
                "header" => {
                    let parsed = text
                        .split_once(':')
                        .map(|(n, v)| (n.trim().to_string(), v.trim().to_string()));
                    match parsed {
                        Some((name, header_value)) if is_settable_header(&name) && is_visible_ascii(&header_value) => {
                            headers.insert(name, header_value);
                        }
                        _ => {
                            return Err(syn::Error::new(
                                value.span(),
                                "header = \"Name: value\": a name that is not a transport header, a visible-ASCII value",
                            ))
                        }
                    }
                }
                "header_arg" => {
                    let parsed = text
                        .split_once('=')
                        .map(|(n, a)| (n.trim().to_string(), a.trim().to_string()));
                    match parsed {
                        Some((name, arg))
                            if is_settable_header(&name)
                                && !arg.is_empty()
                                && arg.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') =>
                        {
                            header_args.insert(name, arg);
                        }
                        _ => {
                            return Err(syn::Error::new(
                                value.span(),
                                "header_arg = \"Header-Name=argName\": the header takes the argument's value",
                            ))
                        }
                    }
                }
                "auth_prefix" => {
                    if !is_visible_ascii(&text) {
                        return Err(syn::Error::new(value.span(), "auth_prefix: visible ASCII"));
                    }
                    auth_prefix = Some(text)
                }
                "auth" => {
                    if !is_known_auth(&text) {
                        return Err(syn::Error::new(
                            value.span(),
                            format!(
                                "unknown auth `{text}`: expected bearer, basic, none or header:<name>"
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

        if auth_prefix.is_some() && !auth.as_deref().is_some_and(|a| a.starts_with("header:")) {
            return Err(syn::Error::new(
                input.span(),
                "auth_prefix only goes with auth = \"header:<name>\"",
            ));
        }
        let key_header = auth
            .as_deref()
            .and_then(|a| a.strip_prefix("header:"))
            .map(str::to_ascii_lowercase);
        if let Some(key_header) = key_header {
            if headers
                .keys()
                .chain(header_args.keys())
                .any(|n| n.to_ascii_lowercase() == key_header)
            {
                return Err(syn::Error::new(
                    input.span(),
                    "a header cannot replace the one that carries the key",
                ));
            }
        }
        let oauth = auth.as_deref() == Some("oauth2_client_credentials");
        if oauth != token_url.is_some() {
            return Err(syn::Error::new(
                input.span(),
                if oauth {
                    "auth = \"oauth2_client_credentials\" needs token_url"
                } else {
                    "token_url only goes with auth = \"oauth2_client_credentials\""
                },
            ));
        }
        Ok(CustomConnectorAttrs {
            id: id.ok_or_else(|| syn::Error::new(input.span(), "id is required"))?,
            display_name_key,
            base_url,
            credential_provider_id,
            auth,
            monthly_quota,
            token_url,
            scopes,
            headers,
            header_args,
            auth_prefix,
            host_key,
        })
    }
}

struct ConnectorOpAttrs {
    method: Option<String>,
    path: Option<String>,
    cache: Option<String>,
    validator: bool,
    /// ADR-0021: every argument name the operation accepts (path, query, header and body keys).
    fields: Option<Vec<String>>,
    /// ADR-0021: what it sends, from [`DATA_CATEGORIES`], shown to the host.
    sends: Option<Vec<String>>,
    /// The `#[custom_connector]` id this operation belongs to. Required as soon as a module
    /// declares two custom connectors; without it the operation goes to the last one declared.
    connector: Option<String>,
}

/// The data a connector operation may say it sends. A closed list: the host reads these, so a
/// free-form label would say whatever its author wanted. `none` is a declaration, not an omission.
pub(crate) const DATA_CATEGORIES: [&str; 10] = [
    "none",
    "property_city",
    "property_address",
    "property_coordinates",
    "stay_dates",
    "guest_count",
    "guest_name",
    "guest_contact",
    "access_codes",
    "module_config",
];

fn comma_list(text: &str) -> Vec<String> {
    text.split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}

impl Parse for ConnectorOpAttrs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        if input.is_empty() {
            return Ok(ConnectorOpAttrs {
                method: None,
                path: None,
                cache: None,
                validator: true,
                fields: None,
                sends: None,
                connector: None,
            });
        }

        let mut method = None;
        let mut path = None;
        let mut cache = None;
        let mut fields = None;
        let mut sends = None;
        let mut connector = None;

        while !input.is_empty() {
            let key: syn::Ident = input.parse()?;
            if key == "validator" && !input.peek(Token![=]) {
                return Ok(ConnectorOpAttrs {
                    method: None,
                    path: None,
                    cache: None,
                    validator: true,
                    fields: None,
                    sends: None,
                    connector: None,
                });
            }

            input.parse::<Token![=]>()?;
            let value: LitStr = input.parse()?;
            let text = value.value();

            match key.to_string().as_str() {
                "method" => method = Some(text),
                "path" => path = Some(text),
                "cache" => cache = Some(text),
                "fields" => fields = Some(comma_list(&text)),
                "connector" => connector = Some(text),
                "sends" => {
                    let categories = comma_list(&text);
                    if let Some(unknown) = categories
                        .iter()
                        .find(|c| !DATA_CATEGORIES.contains(&c.as_str()))
                    {
                        return Err(syn::Error::new(
                            value.span(),
                            format!(
                                "unknown data category `{unknown}`: expected one of {}",
                                DATA_CATEGORIES.join(", ")
                            ),
                        ));
                    }
                    if categories.is_empty() {
                        return Err(syn::Error::new(
                            value.span(),
                            "sends: say `none` when nothing is sent",
                        ));
                    }
                    sends = Some(categories)
                }
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
            fields,
            sends,
            connector,
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
  "auth": {},
  "monthlyQuota": {},
  "tokenUrl": {},
  "scopes": {},
  "headers": {},
  "headerArgs": {},
  "authPrefix": {},
  "hostKey": {}
}}"#,
        serde_json::to_string(&attrs.id).unwrap(),
        serde_json::to_string(&attrs.display_name_key).unwrap(),
        serde_json::to_string(&attrs.base_url).unwrap(),
        serde_json::to_string(&attrs.credential_provider_id).unwrap(),
        serde_json::to_string(&attrs.auth).unwrap(),
        serde_json::to_string(&attrs.monthly_quota).unwrap(),
        serde_json::to_string(&attrs.token_url).unwrap(),
        serde_json::to_string(&attrs.scopes).unwrap(),
        serde_json::to_string(&attrs.headers).unwrap(),
        serde_json::to_string(&attrs.header_args).unwrap(),
        serde_json::to_string(&attrs.auth_prefix).unwrap(),
        attrs.host_key,
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
  "validator": {},
  "fields": {},
  "sends": {},
  "connector": {}
}}"#,
        serde_json::to_string(&fn_name).unwrap(),
        serde_json::to_string(&attrs.method).unwrap(),
        serde_json::to_string(&attrs.path).unwrap(),
        serde_json::to_string(&attrs.cache).unwrap(),
        attrs.validator,
        serde_json::to_string(&attrs.fields).unwrap(),
        serde_json::to_string(&attrs.sends).unwrap(),
        serde_json::to_string(&attrs.connector).unwrap(),
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
            "oauth2_client_credentials",
            "query_appid",
            "query_key",
        ] {
            assert!(is_known_auth(auth), "{auth}");
        }
    }

    #[test]
    fn a_module_sets_any_header_but_the_transport() {
        assert!(super::is_settable_header("Accept"));
        assert!(super::is_settable_header("Accept-Language"));
        for name in ["Host", "content-length", "Transfer-Encoding", "", "X Bad"] {
            assert!(!super::is_settable_header(name), "{name}");
        }
        assert!(super::is_visible_ascii("application/json;version=2.0"));
        assert!(!super::is_visible_ascii("a\r\nb"));
    }

    #[test]
    fn token_urls_are_https_with_nothing_hidden() {
        assert!(super::is_valid_token_url(
            "https://auth.example/oauth/token"
        ));
        for url in [
            "http://auth.example/token",
            "https://u:p@auth.example/token",
            "https://auth.example:8443/token",
            "https://auth.example/token?x=1",
            "https://auth.example/token#f",
            "https:///token",
        ] {
            assert!(!super::is_valid_token_url(url), "{url}");
        }
    }

    #[test]
    fn connector_ids_follow_the_orchestrator_rule() {
        for id in ["viator", "wx", "my_api-2"] {
            assert!(super::is_valid_connector_id(id), "{id}");
        }
        let too_long = "x".repeat(65);
        for id in ["", "Viator", "-wx", "a/b", too_long.as_str()] {
            assert!(!super::is_valid_connector_id(id), "{id}");
        }
    }

    #[test]
    fn anything_else_is_a_build_error() {
        for auth in [
            "Bearer",
            "digest",
            "query:api_key",
            "header:Host",
            "header:Content-Type",
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
