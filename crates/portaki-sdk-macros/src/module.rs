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
}

impl Parse for ModuleAttrs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut attrs = ModuleAttrs {
            id: None,
            display_name_key: None,
            description_key: None,
            author: None,
            version: None,
        };

        while !input.is_empty() {
            let key: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            let value: LitStr = input.parse()?;
            let text = value.value();

            match key.to_string().as_str() {
                "id" => attrs.id = Some(text),
                "display_name_key" => attrs.display_name_key = Some(text),
                "description_key" => attrs.description_key = Some(text),
                "author" => attrs.author = Some(text),
                "version" => attrs.version = Some(text),
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

pub fn expand_invocation(attr: TokenStream) -> TokenStream {
    let attrs = syn::parse_macro_input!(attr as ModuleAttrs);
    emission_tokens(attrs).into()
}

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
    let author = attrs.author.unwrap_or_else(|| "Syntax Labs".to_string());
    let version = attrs.version.unwrap_or_else(default_crate_version);

    let json = format!(
        r#"{{
  "kind": "module",
  "id": {},
  "displayName": {},
  "description": {},
  "author": {{ "name": {} }},
  "version": {},
  "manifestVersion": "1",
  "uiSchema": {{ "host": "1", "guest": "1" }}
}}"#,
        serde_json::to_string(&id).unwrap(),
        serde_json::to_string(&display_name_key).unwrap(),
        serde_json::to_string(&description_key).unwrap(),
        serde_json::to_string(&author).unwrap(),
        serde_json::to_string(&version).unwrap(),
    );

    let emission = write_emission("module", &sanitize_key(&id), &json);

    quote! {
        #emission
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
    use super::default_crate_version;

    #[test]
    fn default_crate_version_uses_compiling_crate_env() {
        let version = default_crate_version();
        assert!(!version.is_empty());
        assert_ne!(version, "0.0.0");
        assert_eq!(version, env!("CARGO_PKG_VERSION"));
    }
}
