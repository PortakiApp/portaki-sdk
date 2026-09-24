//! `portaki_module` / `portaki_module_decl` expansion — module identity + Wasm bootstrap.
//!
//! Parses `key = "value"` attributes (see [`crate::portaki_module`] for the full table).
//! [`expand_invocation`] is the function-like macro path; [`expand`] additionally preserves a
//! decorated `mod` item.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{ItemMod, LitStr, Token};

use crate::emit::{sanitize_key, write_emission};

struct ModuleAttrs {
    id: Option<String>,
    display_name_key: Option<String>,
    description_key: Option<String>,
    author: Option<String>,
    version: Option<String>,
    /// Catalog metadata: `portaki build` writes it to the manifest so no one else has to.
    catalog: serde_json::Map<String, serde_json::Value>,
    /// Compile-time checks that each typed value names a real variant.
    checks: Vec<TokenStream2>,
}

/// Typed keys of `portaki_module!`, their catalogue name and vocabulary.
const TYPED: [(&str, &str, crate::typed::Vocab); 4] = [
    ("icon", "icon", crate::typed::ICON_NAME),
    ("maturity", "maturity", crate::typed::MATURITY),
    ("module_type", "type", crate::typed::MODULE_TYPE),
    ("audience", "audience", crate::typed::MODULE_AUDIENCE),
];

impl Parse for ModuleAttrs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut attrs = ModuleAttrs {
            id: None,
            display_name_key: None,
            description_key: None,
            author: None,
            version: None,
            catalog: serde_json::Map::new(),
            checks: Vec::new(),
        };

        while !input.is_empty() {
            let key: syn::Ident = input.parse()?;
            // The one bare flag: the platform downloads what `scheduled_sync_sources` lists.
            if key == "scheduled_sync_platform_fetch" {
                scheduled_sync(&mut attrs.catalog).insert("platformFetch".into(), true.into());
                input.parse::<Option<Token![,]>>()?;
                continue;
            }
            input.parse::<Token![=]>()?;
            if key == "sort_order" {
                let order: syn::LitInt = input.parse()?;
                attrs
                    .catalog
                    .insert("sortOrder".into(), order.base10_parse::<i64>()?.into());
                input.parse::<Option<Token![,]>>()?;
                continue;
            }
            if let Some((name, wire, vocab)) = TYPED.iter().find(|(name, _, _)| key == name) {
                let typed = crate::typed::parse(input, name, *vocab)?;
                attrs.checks.push(typed.check);
                attrs.catalog.insert((*wire).into(), typed.emitted.into());
                input.parse::<Option<Token![,]>>()?;
                continue;
            }
            let value: LitStr = input.parse()?;
            let text = value.value();

            match key.to_string().as_str() {
                "id" => attrs.id = Some(text),
                "display_name_key" => attrs.display_name_key = Some(text),
                "description_key" => attrs.description_key = Some(text),
                "author" => attrs.author = Some(text),
                "version" => attrs.version = Some(text),
                "author_url" if is_https_url(&text) => {
                    attrs.catalog.insert("authorUrl".into(), text.into());
                }
                "author_url" => {
                    return Err(syn::Error::new(
                        value.span(),
                        "author_url must be an https:// URL",
                    ));
                }
                "feeds" if !is_module_id(&text) => {
                    return Err(syn::Error::new(
                        value.span(),
                        "feeds takes a module id: lowercase, digits and dashes, e.g. \"access-guide\"",
                    ));
                }
                // Repeats: each one a module this one supplies from behind the scenes. What it
                // supplies is read from the i18n key `feeds.<module>`.
                "feeds" => {
                    attrs
                        .catalog
                        .entry("feeds")
                        .or_insert_with(|| serde_json::Value::Array(Vec::new()))
                        .as_array_mut()
                        .expect("list")
                        .push(text.into());
                }
                "scheduled_sync_sources" => {
                    scheduled_sync(&mut attrs.catalog).insert("sourcesQuery".into(), text.into());
                }
                "scheduled_sync_apply" => {
                    scheduled_sync(&mut attrs.catalog).insert("applyQuery".into(), text.into());
                }
                other => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!("unknown portaki_module attribute: {other}"),
                    ));
                }
            }

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(attrs)
    }
}

/// `^[a-z][a-z0-9-]*$` — the schema's module id pattern.
fn is_module_id(id: &str) -> bool {
    let mut chars = id.chars();
    matches!(chars.next(), Some(first) if first.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn is_https_url(url: &str) -> bool {
    url.strip_prefix("https://")
        .is_some_and(|rest| !rest.is_empty() && !rest.contains(char::is_whitespace))
}

/// The catalogue's `hostScheduledSync`, created on first use.
fn scheduled_sync(
    catalog: &mut serde_json::Map<String, serde_json::Value>,
) -> &mut serde_json::Map<String, serde_json::Value> {
    catalog
        .entry("hostScheduledSync")
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()))
        .as_object_mut()
        .expect("object")
}

/// Expands `portaki_module!(…)` — emission + Wasm shims only.
pub fn expand_invocation(attr: TokenStream) -> TokenStream {
    let attrs = syn::parse_macro_input!(attr as ModuleAttrs);
    emission_tokens(attrs).into()
}

/// Expands `#[portaki_module(…)] mod …` — emission + Wasm shims + passthrough `mod` item.
pub fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    let module_item = syn::parse_macro_input!(item as ItemMod);
    let attrs = syn::parse_macro_input!(attr as ModuleAttrs);
    let emission = emission_tokens(attrs);
    let output: TokenStream2 = quote! {
        #emission
        #module_item
    };
    output.into()
}

fn emission_tokens(attrs: ModuleAttrs) -> TokenStream2 {
    let id = attrs.id.unwrap_or_else(|| "unknown".to_string());
    let display_name_key = attrs
        .display_name_key
        .unwrap_or_else(|| "module.displayName".to_string());
    let description_key = attrs
        .description_key
        .unwrap_or_else(|| "module.description".to_string());
    let author = attrs.author.unwrap_or_else(|| "Portaki".to_string());
    let version = attrs.version.unwrap_or_else(default_crate_version);

    let json = serde_json::to_string_pretty(&serde_json::json!({
        "kind": "module",
        "id": id,
        "displayName": display_name_key,
        "description": description_key,
        "author": { "name": author },
        "version": version,
        "manifestVersion": "1",
        "uiSchema": { "host": "1", "guest": "1" },
        "catalog": attrs.catalog,
    }))
    .unwrap();

    let emission = write_emission("module", &sanitize_key(&id), &json);
    let checks = &attrs.checks;

    quote! {
        #emission
        #(#checks)*
        #[cfg(target_arch = "wasm32")]
        mod __portaki_wasm_getrandom {
            #[no_mangle]
            unsafe extern "Rust" fn __getrandom_v03_custom(
                dest: *mut u8,
                len: usize,
            ) -> Result<(), ::portaki_sdk::host::wasm_getrandom::getrandom::Error> {
                let buf = unsafe {
                    core::ptr::write_bytes(dest, 0, len);
                    core::slice::from_raw_parts_mut(dest, len)
                };
                ::portaki_sdk::host::wasm_getrandom::fill(buf)
            }
        }

        #[cfg(target_arch = "wasm32")]
        mod __portaki_wasm_exports {
            use extism_pdk::{FnResult, plugin_fn};

            #[plugin_fn]
            pub fn portaki_query(input: String) -> FnResult<String> {
                Ok(::portaki_sdk::wasm::dispatch::dispatch_query_json(&input)?)
            }

            #[plugin_fn]
            pub fn portaki_command(input: String) -> FnResult<String> {
                Ok(::portaki_sdk::wasm::dispatch::dispatch_command_json(&input)?)
            }
        }
    }
}

/// Version of the crate being compiled (module), not the proc-macro crate.
///
/// `env!("CARGO_PKG_VERSION")` is wrong here: it is evaluated when `portaki-sdk-macros` is built
/// (SDK workspace version), so every module would inherit e.g. `0.1.0`.
fn default_crate_version() -> String {
    std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".to_string())
}

#[cfg(test)]
mod tests {
    use super::{default_crate_version, ModuleAttrs};

    #[test]
    fn catalog_metadata_is_collected_under_its_manifest_names() {
        let attrs: ModuleAttrs = syn::parse_str(
            r#"id = "issue-report", icon = IconName::DangerTriangle, maturity = Maturity::Stable,
               module_type = ModuleType::Official, author_url = "https://portaki.app", sort_order = 90,"#,
        )
        .unwrap();
        assert_eq!(
            serde_json::Value::Object(attrs.catalog),
            serde_json::json!({
                "icon": "IconName::DangerTriangle",
                "maturity": "Maturity::Stable",
                "type": "ModuleType::Official",
                "authorUrl": "https://portaki.app",
                "sortOrder": 90,
            })
        );
        assert!(syn::parse_str::<ModuleAttrs>(r#"sort_order = "90""#).is_err());
    }

    #[test]
    fn a_host_module_declares_its_sync_and_what_it_feeds() {
        let attrs: ModuleAttrs = syn::parse_str(
            r#"id = "ical-sync", audience = ModuleAudience::Host, scheduled_sync_platform_fetch,
               scheduled_sync_sources = "listSources", scheduled_sync_apply = "applyFeeds",
               feeds = "access-guide", feeds = "wifi-guest""#,
        )
        .unwrap();
        assert_eq!(
            serde_json::Value::Object(attrs.catalog),
            serde_json::json!({
                "audience": "ModuleAudience::Host",
                "hostScheduledSync": {
                    "platformFetch": true,
                    "sourcesQuery": "listSources",
                    "applyQuery": "applyFeeds",
                },
                "feeds": ["access-guide", "wifi-guest"],
            })
        );
        assert!(syn::parse_str::<ModuleAttrs>(r#"audience = "host""#).is_err());
        assert!(syn::parse_str::<ModuleAttrs>(r#"author_url = "http://x.fr""#).is_err());
        assert!(syn::parse_str::<ModuleAttrs>(r#"feeds = "Access_Guide""#).is_err());
    }

    #[test]
    fn default_crate_version_uses_compiling_crate_env() {
        let version = default_crate_version();
        assert!(!version.is_empty());
        assert_ne!(version, "0.0.0");
        assert_eq!(version, env!("CARGO_PKG_VERSION"));
    }
}
