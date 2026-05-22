use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{ItemConst, LitStr, Token};

use crate::emit::{sanitize_key, write_emission};

struct CapabilityAttrs {
    optional: bool,
    purpose_key: Option<String>,
    fallback_key: Option<String>,
}

impl Parse for CapabilityAttrs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut optional = false;
        let mut purpose_key = None;
        let mut fallback_key = None;

        while !input.is_empty() {
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }

            if input.peek(syn::Ident) {
                let lookahead = input.fork();
                let key: syn::Ident = lookahead.parse()?;
                if key == "optional" && !lookahead.peek(Token![=]) {
                    input.parse::<syn::Ident>()?;
                    optional = true;
                    continue;
                }
                if key == "required" && !lookahead.peek(Token![=]) {
                    input.parse::<syn::Ident>()?;
                    optional = false;
                    continue;
                }
            }

            let key: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            let value: LitStr = input.parse()?;
            let text = value.value();

            match key.to_string().as_str() {
                "purpose_key" => purpose_key = Some(text),
                "fallback_key" => fallback_key = Some(text),
                other => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!("unknown #[capability] attribute: {other}"),
                    ));
                }
            }
        }

        Ok(CapabilityAttrs {
            optional,
            purpose_key,
            fallback_key,
        })
    }
}

pub fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    let const_item = syn::parse_macro_input!(item as ItemConst);
    let attrs = syn::parse_macro_input!(attr as CapabilityAttrs);
    let const_name = const_item.ident.to_string();

    let capability_id = match &*const_item.expr {
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(lit),
            ..
        }) => lit.value(),
        _ => String::new(),
    };

    let json = format!(
        r#"{{
  "kind": "capability",
  "const": {},
  "id": {},
  "optional": {},
  "purposeKey": {},
  "fallbackKey": {}
}}"#,
        serde_json::to_string(&const_name).unwrap(),
        serde_json::to_string(&capability_id).unwrap(),
        attrs.optional,
        serde_json::to_string(&attrs.purpose_key).unwrap(),
        serde_json::to_string(&attrs.fallback_key).unwrap(),
    );

    let emission = write_emission("capability", &sanitize_key(&capability_id), &json);
    let output: TokenStream2 = quote! {
        #emission
        #const_item
    };

    output.into()
}
