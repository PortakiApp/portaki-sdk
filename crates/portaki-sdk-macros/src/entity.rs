//! `entity` and `entity_indexes` expansion — Atlas entity schemas and index metadata.
//!
//! [`expand_entity`] serializes named struct fields to JSON. [`expand_entity_indexes`] parses the
//! const array literal at compile time (see [`crate::entity_indexes`] for spatial vs field rules).

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{Fields, ItemConst, ItemStruct, LitInt, Token};

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

/// Expands `#[entity(schema_version = N)]` on a struct.
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

    let emission = write_emission("entity", &sanitize_key(&name), &json);
    let output: TokenStream2 = quote! {
        #emission
        #struct_item
    };

    output.into()
}

/// Expands `#[entity_indexes(EntityType)]` on a `const` string-slice array.
pub fn expand_entity_indexes(attr: TokenStream, item: TokenStream) -> TokenStream {
    let const_item = syn::parse_macro_input!(item as ItemConst);
    let entity_type = syn::parse_macro_input!(attr as syn::Type);
    let entity_name = type_name(&entity_type);

    let index_fields = match parse_index_field_names(&const_item.expr) {
        Ok(fields) => fields,
        Err(error) => return error.to_compile_error().into(),
    };

    let indexes_json = indexes_json(&index_fields);
    let json = format!(
        r#"{{
  "kind": "entity_indexes",
  "entity": {},
  "indexes": {indexes_json}
}}"#,
        serde_json::to_string(&entity_name).unwrap(),
    );

    let emission = write_emission("entity_indexes", &sanitize_key(&entity_name), &json);
    let output: TokenStream2 = quote! {
        #emission
        #const_item
    };

    output.into()
}

fn indexes_json(fields: &[String]) -> String {
    if fields.is_empty() {
        return "[]".to_string();
    }

    if fields.len() >= 2
        && fields.contains(&"lat".to_string())
        && fields.contains(&"lng".to_string())
    {
        return serde_json::json!([{
            "kind": "spatial",
            "fields": fields,
        }])
        .to_string();
    }

    let entries: Vec<serde_json::Value> = fields
        .iter()
        .map(|field| serde_json::json!({ "kind": "field", "field": field }))
        .collect();
    serde_json::to_string(&entries).unwrap()
}

fn parse_index_field_names(expr: &syn::Expr) -> syn::Result<Vec<String>> {
    let array = match expr {
        syn::Expr::Array(array) => array,
        syn::Expr::Reference(reference) => match &*reference.expr {
            syn::Expr::Array(array) => array,
            other => {
                return Err(syn::Error::new(
                    other.span(),
                    "#[entity_indexes] value must be a string array literal like &[\"lat\", \"lng\"]",
                ));
            }
        },
        other => {
            return Err(syn::Error::new(
                other.span(),
                "#[entity_indexes] value must be a string array literal like &[\"lat\", \"lng\"]",
            ));
        }
    };

    let mut fields = Vec::new();
    for element in &array.elems {
        match element {
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(lit),
                ..
            }) => fields.push(lit.value()),
            other => {
                return Err(syn::Error::new(
                    other.span(),
                    "entity index entries must be string literals",
                ));
            }
        }
    }
    Ok(fields)
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

#[cfg(test)]
mod tests {
    use super::{indexes_json, parse_index_field_names};
    use syn::parse_quote;

    #[test]
    fn parse_index_field_names_from_array() {
        let expr: syn::Expr = parse_quote! { &["lat", "lng"] };
        let fields = parse_index_field_names(&expr).unwrap();
        assert_eq!(fields, vec!["lat", "lng"]);
    }

    #[test]
    fn indexes_json_spatial_for_lat_lng_pair() {
        let json = indexes_json(&["lat".into(), "lng".into()]);
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value[0]["kind"], "spatial");
        assert_eq!(value[0]["fields"], serde_json::json!(["lat", "lng"]));
    }

    #[test]
    fn indexes_json_field_for_single_column() {
        let json = indexes_json(&["category".into()]);
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value[0]["kind"], "field");
        assert_eq!(value[0]["field"], "category");
    }
}
