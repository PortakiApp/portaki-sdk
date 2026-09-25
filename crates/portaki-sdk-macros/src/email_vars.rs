//! `email_vars` expansion — the variables a module gives Portaki guest emails, per template.
//!
//! The attribute sits on the function that computes the values. Generated next to it:
//!
//! - the `emailContext` query the platform calls, which asks the function only for a declared
//!   template and checks its answer against the declaration (`portaki_sdk::email::serve`);
//! - a `const` assertion per (template, variable): a variable its template does not render does
//!   not compile — the catalogue is `portaki_sdk::email::EmailVar`, not a list copied here;
//! - the emission `email_vars`, which `portaki build` writes as the manifest's `emailVars`.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use serde_json::json;
use syn::parse::{Parse, ParseStream};
use syn::{ItemFn, Token};

use crate::emit::{sanitize_key, write_emission};
use crate::typed::{self, Typed, EMAIL_TEMPLATE_KEY as TEMPLATE, EMAIL_VAR as VAR};

/// The query the platform calls for email variables.
const QUERY: &str = "emailContext";

/// `Arrival | ArrivalDay => [WifiName, HostPhone]`, comma-separated.
struct Declaration {
    entries: Vec<(Vec<Typed>, Vec<Typed>)>,
}

impl Parse for Declaration {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut entries = Vec::new();
        while !input.is_empty() {
            let mut templates = vec![typed::parse(input, "template", TEMPLATE)?];
            while input.parse::<Option<Token![|]>>()?.is_some() {
                templates.push(typed::parse(input, "template", TEMPLATE)?);
            }
            input.parse::<Token![=>]>()?;
            let content;
            syn::bracketed!(content in input);
            let mut vars = Vec::new();
            while !content.is_empty() {
                vars.push(typed::parse(&content, "variable", VAR)?);
                content.parse::<Option<Token![,]>>()?;
            }
            if vars.is_empty() {
                return Err(content.error("list the variables: [WifiName, …]"));
            }
            entries.push((templates, vars));
            input.parse::<Option<Token![,]>>()?;
        }
        if entries.is_empty() {
            return Err(typed::error(
                "declare what the module gives: #[email_vars(Arrival | ArrivalDay => [WifiName])]",
            ));
        }
        Ok(Declaration { entries })
    }
}

/// Expands `#[portaki_sdk::email_vars(…)]` on `fn(Context, EmailContextArgs) -> Result<EmailVars>`.
pub fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    let declaration = syn::parse_macro_input!(attr as Declaration);
    let function = syn::parse_macro_input!(item as ItemFn);
    match expand_fn(declaration, function) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn expand_fn(declaration: Declaration, function: ItemFn) -> syn::Result<TokenStream2> {
    // Per template, in source order: (emitted template, [(emitted var, var path)]).
    let mut by_template: Vec<(String, TokenStream2, Vec<Typed>)> = Vec::new();
    let mut checks = Vec::new();
    for (templates, vars) in declaration.entries {
        for template in templates {
            if by_template.iter().any(|(t, _, _)| *t == template.emitted) {
                return Err(typed::error(format!(
                    "{} is declared twice — one entry per template",
                    template.emitted
                )));
            }
            checks.push(template.check.clone());
            let mut listed: Vec<Typed> = Vec::new();
            for var in &vars {
                if listed.iter().any(|v| v.emitted == var.emitted) {
                    return Err(typed::error(format!(
                        "{} is listed twice for {}",
                        var.emitted, template.emitted
                    )));
                }
                checks.push(var.check.clone());
                let (var_path, template_path) = (&var.path, &template.path);
                let message = format!(
                    "{} is not rendered by {} — see EmailVar::templates",
                    var.emitted, template.emitted
                );
                checks.push(quote! {
                    const _: () = ::core::assert!(#var_path.renders_in(#template_path), #message);
                });
                listed.push(Typed {
                    emitted: var.emitted.clone(),
                    check: TokenStream2::new(),
                    path: var.path.clone(),
                });
            }
            by_template.push((template.emitted, template.path, listed));
        }
    }

    let emission = write_emission(
        "email_vars",
        &sanitize_key(QUERY),
        &serde_json::to_string_pretty(&json!({
            "kind": "email_vars",
            "declared": by_template.iter().map(|(template, _, vars)| json!({
                "template": template,
                "vars": vars.iter().map(|v| v.emitted.clone()).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
        }))
        .unwrap(),
    );

    let user_fn = &function.sig.ident;
    let shim_ident = format_ident!("portaki_email_context_{}", user_fn);
    let declared_ident = format_ident!(
        "__PORTAKI_EMAIL_VARS_{}",
        user_fn.to_string().to_uppercase()
    );
    let entries = by_template.iter().map(|(_, template, vars)| {
        let vars = vars.iter().map(|v| &v.path);
        quote! { (#template, &[#(#vars),*]) }
    });
    let shim: ItemFn = syn::parse_quote! {
        fn #shim_ident(
            ctx: ::portaki_sdk::context::Context,
            args: ::portaki_sdk::email::EmailContextArgs,
        ) -> ::portaki_sdk::error::Result<::serde_json::Value> {
            ::portaki_sdk::email::serve(#declared_ident, ctx, args, #user_fn)
        }
    };
    let shim_name = shim_ident.to_string();
    let query_emission = write_emission(
        "query",
        &sanitize_key(QUERY),
        &serde_json::to_string_pretty(&json!({
            "kind": "query",
            "name": QUERY,
            "fn": shim_name,
            "guest": false,
        }))
        .unwrap(),
    );
    let registration = crate::wasm_handler::register_query(QUERY, &shim_name, &shim);
    let dispatch = format_ident!("__portaki_shim_{}", shim_ident);

    Ok(quote! {
        #emission
        #query_emission
        #(#checks)*

        #function

        #[allow(non_upper_case_globals)]
        const #declared_ident: ::portaki_sdk::email::DeclaredEmailVars = &[#(#entries),*];

        #shim
        #registration

        #[cfg(not(target_arch = "wasm32"))]
        ::portaki_sdk::inventory::submit! {
            ::portaki_sdk::email::EmailVarsDeclaration {
                declared: #declared_ident,
                dispatch: #dispatch,
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    fn expanded(attr: &str) -> syn::Result<String> {
        let declaration: Declaration = syn::parse_str(attr)?;
        let function: ItemFn = parse_quote! {
            pub fn vars(ctx: Context, args: EmailContextArgs) -> Result<EmailVars> { todo!() }
        };
        expand_fn(declaration, function).map(|tokens| tokens.to_string())
    }

    #[test]
    fn templates_share_a_list_and_each_pair_is_checked_at_compile_time() {
        let out = expanded(
            "Arrival | EmailTemplateKey::ArrivalDay => [WifiName], StayLink => [WifiName,]",
        )
        .unwrap();
        assert_eq!(out.matches("renders_in").count(), 3, "{out}");
        assert!(out.contains("\"emailContext\""), "{out}");
        assert!(out.contains("portaki_sdk :: email :: serve"), "{out}");
        assert!(out.contains("EmailVarsDeclaration"), "{out}");
        assert!(
            out.contains(":: portaki_sdk :: email :: EmailVar :: WifiName"),
            "{out}"
        );
    }

    #[test]
    fn wrong_declarations_do_not_compile() {
        for attr in [
            "",
            "Arrival => []",
            "Arrival => [WifiName], Arrival => [HostPhone]",
            "Arrival => [WifiName, WifiName]",
            "\"arrival\" => [WifiName]",
            "Arrival => [\"wifiName\"]",
            "Arrival => [EmailTemplateKey::WifiName]",
        ] {
            assert!(expanded(attr).is_err(), "{attr}");
        }
    }
}
