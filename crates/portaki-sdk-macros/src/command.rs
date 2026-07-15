//! `command` expansion — mutating gateway operations (same shape as `query`).

use proc_macro::TokenStream;

/// Expands `#[command(name = "…")]` on a handler function.
pub fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    let function_item = syn::parse_macro_input!(item as syn::ItemFn);
    let attrs = syn::parse_macro_input!(attr as command_attrs::NamedOpAttrs);
    let fn_name = function_item.sig.ident.to_string();

    let json = format!(
        r#"{{
  "kind": "command",
  "name": {},
  "fn": {}
}}"#,
        serde_json::to_string(&attrs.name).unwrap(),
        serde_json::to_string(&fn_name).unwrap(),
    );

    let emission =
        crate::emit::write_emission("command", &crate::emit::sanitize_key(&attrs.name), &json);
    let wasm_registration =
        crate::wasm_handler::register_command(&attrs.name, &fn_name, &function_item);
    let output = quote::quote! {
        #emission
        #function_item
        #wasm_registration
    };

    output.into()
}

mod command_attrs {
    use syn::parse::{Parse, ParseStream};
    use syn::{LitStr, Token};

    pub struct NamedOpAttrs {
        pub name: String,
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
}
