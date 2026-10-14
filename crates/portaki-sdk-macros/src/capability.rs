//! `capability` expansion — required/optional host capability dependencies, and provided ones
//! with the config keys they share (`shares = ["key", …]`), the guest path consumers open
//! (`path = "…"`) and the commands they may call (`commands = ["…"]`).
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
    shares: Vec<String>,
    path: Option<String>,
    commands: Vec<String>,
}

impl Parse for CapabilityAttrs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut optional = false;
        let mut provided = false;
        let mut capability_id = None;
        let mut purpose_key = None;
        let mut fallback_key = None;
        let mut shares = Vec::new();
        let mut path = None;
        let mut commands = Vec::new();

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
            if key == "shares" || key == "commands" {
                let list;
                syn::bracketed!(list in input);
                let keys = list.parse_terminated(|p| p.parse::<LitStr>(), Token![,])?;
                let keys = keys.iter().map(LitStr::value).collect();
                if key == "shares" {
                    shares = keys;
                } else {
                    commands = keys;
                }
                continue;
            }
            let value: LitStr = input.parse()?;
            let text = value.value();

            match key.to_string().as_str() {
                "id" => capability_id = Some(text),
                "purpose_key" => purpose_key = Some(text),
                "fallback_key" => fallback_key = Some(text),
                "path" => path = Some(text),
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
            shares,
            path,
            commands,
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

    let provider_only = [
        (!attrs.shares.is_empty(), "shares = [...]"),
        (attrs.path.is_some(), "path = \"...\""),
        (!attrs.commands.is_empty(), "commands = [...]"),
    ];
    if let Some((_, attribute)) = provider_only
        .iter()
        .find(|(set, _)| *set && !attrs.provided)
    {
        return syn::Error::new(
            const_item.ident.span(),
            format!("#[capability({attribute})] only applies to a provided capability"),
        )
        .to_compile_error()
        .into();
    }

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
  "fallbackKey": {},
  "shares": {},
  "path": {},
  "commands": {}
}}"#,
        serde_json::to_string(&const_name).unwrap(),
        serde_json::to_string(&capability_id).unwrap(),
        attrs.optional,
        attrs.provided,
        serde_json::to_string(&attrs.purpose_key).unwrap(),
        serde_json::to_string(&attrs.fallback_key).unwrap(),
        serde_json::to_string(&attrs.shares).unwrap(),
        serde_json::to_string(&attrs.path).unwrap(),
        serde_json::to_string(&attrs.commands).unwrap(),
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
