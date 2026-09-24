//! `nav` expansion — a host dashboard entry that no surface renders.
//!
//! Most entries come from `#[surface(host, placement = …)]`. A few are links the dashboard draws
//! itself (checklist's timeline task): they have no render function to sit on, so this attribute
//! goes on any item — the item is passed through untouched.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use syn::parse::{Parse, ParseStream};
use syn::{LitStr, Token};

use crate::emit::{sanitize_key, write_emission};

const KEYS: [&str; 5] = ["placement", "path", "label_key", "icon", "design_id"];

pub(crate) struct NavAttrs {
    fields: serde_json::Map<String, serde_json::Value>,
}

impl Parse for NavAttrs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut fields = serde_json::Map::new();
        while !input.is_empty() {
            let key: syn::Ident = input.parse()?;
            let name = key.to_string();
            if !KEYS.contains(&name.as_str()) {
                return Err(syn::Error::new(
                    key.span(),
                    format!(
                        "unknown #[nav] attribute: {name} — expected one of {}",
                        KEYS.join(", ")
                    ),
                ));
            }
            input.parse::<Token![=]>()?;
            let value = input.parse::<LitStr>()?.value();
            if fields.insert(name.clone(), value.into()).is_some() {
                return Err(syn::Error::new(
                    key.span(),
                    format!("{name} is given twice"),
                ));
            }
            input.parse::<Option<Token![,]>>()?;
        }
        for required in ["placement", "path"] {
            if !fields.contains_key(required) {
                return Err(input.error(format!("#[nav] needs {required} = \"…\"")));
            }
        }
        Ok(NavAttrs { fields })
    }
}

/// Expands `#[nav(placement = "…", path = "…", …)]` — emission only, item unchanged.
pub fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attrs = syn::parse_macro_input!(attr as NavAttrs);
    let mut entry = serde_json::Value::Object(attrs.fields);
    let key = format!(
        "{}_{}",
        entry["path"].as_str().unwrap_or_default(),
        entry["placement"].as_str().unwrap_or_default()
    );
    entry["kind"] = "nav".into();
    let emission = write_emission(
        "nav",
        &sanitize_key(&key),
        &serde_json::to_string_pretty(&entry).unwrap(),
    );
    let item: TokenStream2 = item.into();
    quote::quote! { #emission #item }.into()
}

#[cfg(test)]
mod tests {
    use super::NavAttrs;

    #[test]
    fn an_entry_needs_a_placement_and_a_path() {
        let attrs: NavAttrs = syn::parse_str(
            r#"placement = "workspace-timeline-task", path = "tasks", label_key = "nav.tasks", icon = "sparkles""#,
        )
        .unwrap();
        assert_eq!(attrs.fields["path"], "tasks");
        assert!(syn::parse_str::<NavAttrs>(r#"placement = "x""#).is_err());
        assert!(syn::parse_str::<NavAttrs>(r#"placement = "x", path = "y", role = "z""#).is_err());
    }
}
