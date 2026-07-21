//! `capability` expansion — required/optional host capability dependencies.
//!
//! Resolves capability id from `id = "…"` or a string-literal const initializer. Cross-const
//! references require an explicit `id` attribute.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{ItemConst, LitStr, Token};

use crate::emit::{sanitize_key, write_emission};

struct CapabilityAttrs {
    optional: bool,
    provided: bool,
    capability_id: Option<String>,
    purpose_key: Option<String>,
    fallback_key: Option<String>,
}

impl Parse for CapabilityAttrs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut optional = false;
        let mut provided = false;
        let mut capability_id = None;
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
                if key == "provided" && !lookahead.peek(Token![=]) {
                    input.parse::<syn::Ident>()?;
                    provided = true;
                    continue;
                }
            }

            let key: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            let value: LitStr = input.parse()?;
            let text = value.value();

            match key.to_string().as_str() {
                "id" => capability_id = Some(text),
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
            provided,
            capability_id,
            purpose_key,
            fallback_key,
        })
    }
}

/// Expands `#[capability(required|optional, …)]` on a `pub const` capability binding.
pub fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    let const_item = syn::parse_macro_input!(item as ItemConst);
    let attrs = syn::parse_macro_input!(attr as CapabilityAttrs);
    let const_name = const_item.ident.to_string();

    let capability_id = attrs
        .capability_id
        .or_else(|| capability_id_from_expr(&const_item.expr))
        .unwrap_or_default();

    if capability_id.is_empty() {
        return syn::Error::new(
            const_item.expr.span(),
            "#[capability] requires a string literal value or an explicit id = \"...\" attribute",
        )
        .to_compile_error()
        .into();
    }

    let json = format!(
        r#"{{
  "kind": "capability",
  "const": {},
  "id": {},
  "optional": {},
  "provided": {},
  "purposeKey": {},
  "fallbackKey": {}
}}"#,
        serde_json::to_string(&const_name).unwrap(),
        serde_json::to_string(&capability_id).unwrap(),
        attrs.optional,
        attrs.provided,
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

fn capability_id_from_expr(expr: &syn::Expr) -> Option<String> {
    match expr {
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(lit),
            ..
        }) => Some(lit.value()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::capability_id_from_expr;
    use syn::parse_quote;

    #[test]
    fn capability_id_from_string_literal() {
        let expr: syn::Expr = parse_quote! { "core.storage" };
        assert_eq!(
            capability_id_from_expr(&expr).as_deref(),
            Some("core.storage")
        );
    }

    #[test]
    fn capability_id_from_path_is_none() {
        let expr: syn::Expr = parse_quote! { OTHER_CONST };
        assert!(capability_id_from_expr(&expr).is_none());
    }
}
