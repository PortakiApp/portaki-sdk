use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Fields, ItemStruct, LitInt, Token};

use crate::emit::{sanitize_key, write_emission};

struct EntityAttrs {
    schema_version: u32,
}

impl Parse for EntityAttrs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut schema_version = None;

        while !input.is_empty() {
            let key: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            if key == "schema_version" {
                let value: LitInt = input.parse()?;
                schema_version = Some(value.base10_parse()?);
            } else {
                return Err(syn::Error::new(
                    key.span(),
                    format!("unknown #[entity] attribute: {key}"),
                ));
            }

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(EntityAttrs {
            schema_version: schema_version.ok_or_else(|| {
                syn::Error::new(input.span(), "schema_version is required on #[entity]")
            })?,
        })
    }
}

pub fn expand_entity(attr: TokenStream, item: TokenStream) -> TokenStream {
    let struct_item = syn::parse_macro_input!(item as ItemStruct);
    let attrs = syn::parse_macro_input!(attr as EntityAttrs);
    let name = struct_item.ident.to_string();
    let fields = extract_fields(&struct_item);

    let json = format!(
        r#"{{
  "kind": "entity",
  "name": {},
  "schemaVersion": {},
  "fields": {}
}}"#,
        serde_json::to_string(&name).unwrap(),
        attrs.schema_version,
        serde_json::to_string(&fields).unwrap(),
    );

    let emission = write_emission("entity", &sanitize_key(&name), quote! { #json });
    let output: TokenStream2 = quote! {
        #emission
        #struct_item
    };

    output.into()
}

pub fn expand_entity_indexes(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = syn::parse_macro_input!(item as syn::Item);
    let entity_type = syn::parse_macro_input!(attr as syn::Type);

    let entity_name = type_name(&entity_type);
    let json = format!(
        r#"{{
  "kind": "entity_indexes",
  "entity": {},
  "indexes": []
}}"#,
        serde_json::to_string(&entity_name).unwrap(),
    );

    let emission = write_emission(
        "entity_indexes",
        &sanitize_key(&entity_name),
        quote! { #json },
    );
    let output: TokenStream2 = quote! {
        #emission
        #item
    };

    output.into()
}

fn type_name(ty: &syn::Type) -> String {
    match ty {
        syn::Type::Path(path) => path
            .path
            .get_ident()
            .map(|ident| ident.to_string())
            .unwrap_or_else(|| "Unknown".to_string()),
        _ => "Unknown".to_string(),
    }
}

fn extract_fields(item: &ItemStruct) -> Vec<serde_json::Value> {
    let fields = match &item.fields {
        Fields::Named(fields) => &fields.named,
        _ => return Vec::new(),
    };

    fields
        .iter()
        .filter_map(|field| {
            let name = field.ident.as_ref()?.to_string();
            let ty = quote!(#field.ty).to_string();
            Some(serde_json::json!({
                "name": name,
                "type": ty,
            }))
        })
        .collect()
}
