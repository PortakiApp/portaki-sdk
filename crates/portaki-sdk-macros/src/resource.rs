//! `#[choice]`, `#[item]`, `#[collection]` — declared resources (feature `unstable` of the SDK).
//!
//! Each writes the Rust the SDK runtime reads (`Choice`, `Item` / `Meta`, the collection's
//! `load` / `save` and its generated operations) and an emission `portaki build` turns into the
//! manifest's `resources[]`.

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use serde_json::{json, Value};
use syn::spanned::Spanned;
use syn::{Fields, GenericArgument, ItemEnum, ItemStruct, PathArguments, Type};

use crate::emit::{sanitize_key, write_emission};
use crate::params::{rename_variant, serde_of};

// ── #[choice] ────────────────────────────────────────────────────────────────────────────────

/// `#[choice(label = "rule.status")]` on a unit enum.
pub fn expand_choice(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut item = syn::parse_macro_input!(item as ItemEnum);
    let mut label: Option<String> = None;
    let parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("label") {
            label = Some(meta.value()?.parse::<syn::LitStr>()?.value());
            Ok(())
        } else {
            Err(meta.error("#[choice] takes label = \"<bundle prefix>\""))
        }
    });
    syn::parse_macro_input!(attr with parser);
    let Some(label) = label else {
        return syn::Error::new(
            Span::call_site(),
            "#[choice] needs label = \"<bundle prefix>\"",
        )
        .to_compile_error()
        .into();
    };
    match choice(&mut item, &label) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn choice(item: &mut ItemEnum, label: &str) -> syn::Result<TokenStream2> {
    let ident = item.ident.clone();
    let mut wire_list = Vec::new();
    let mut variants = Vec::new();
    let mut wires = Vec::new();
    let mut icons = Vec::new();
    let has_default = item
        .variants
        .iter()
        .any(|v| v.attrs.iter().any(|a| a.path().is_ident("default")));
    for (index, variant) in item.variants.iter_mut().enumerate() {
        if !matches!(variant.fields, Fields::Unit) {
            return Err(syn::Error::new(
                variant.span(),
                "a choice is a unit variant",
            ));
        }
        let wire = serde_of(&variant.attrs)
            .rename
            .unwrap_or_else(|| rename_variant(&variant.ident.to_string(), Some("snake_case")));
        let name = variant.ident.clone();
        if let Some(position) = variant.attrs.iter().position(|a| a.path().is_ident("icon")) {
            let icon: syn::Expr = variant.attrs.remove(position).parse_args()?;
            icons.push(quote! { Self::#name => ::core::option::Option::Some(#icon), });
        }
        if index == 0 && !has_default {
            variant.attrs.push(syn::parse_quote!(#[default]));
        }
        variants.push(quote! { Self::#name });
        wires.push(quote! { Self::#name => #wire, });
        wire_list.push(wire);
    }
    let emission = write_emission(
        "params",
        &sanitize_key(&ident.to_string()),
        &json!({ "kind": "params", "name": ident.to_string(), "enum": wire_list, "labelPrefix": label })
            .to_string(),
    );
    Ok(quote! {
        #emission
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ::serde::Serialize, ::serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
        #item

        impl ::portaki_sdk::resource::Choice for #ident {
            const ALL: &'static [Self] = &[#(#variants),*];
            const LABEL_PREFIX: &'static str = #label;
            fn wire(self) -> &'static str {
                match self { #(#wires)* }
            }
            fn icon(self) -> ::core::option::Option<::portaki_sdk::vocab::IconName> {
                #[allow(unreachable_patterns)]
                match self { #(#icons)* _ => ::core::option::Option::None }
            }
        }
    })
}

// ── #[item] ──────────────────────────────────────────────────────────────────────────────────

/// `#[item(key = "title")]` on a row type, `#[item(meta)]` on a collection's settings.
pub fn expand_item(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut item = syn::parse_macro_input!(item as ItemStruct);
    let mut key: Option<String> = None;
    let mut meta = false;
    let parser = syn::meta::parser(|m| {
        if m.path.is_ident("key") {
            key = Some(m.value()?.parse::<syn::LitStr>()?.value());
            Ok(())
        } else if m.path.is_ident("meta") {
            meta = true;
            Ok(())
        } else {
            Err(m.error("#[item] takes key = \"<field>\", or meta"))
        }
    });
    syn::parse_macro_input!(attr with parser);
    match item_tokens(&mut item, key, meta) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

struct Declared {
    spec: TokenStream2,
    schema: Value,
}

fn item_tokens(
    item: &mut ItemStruct,
    key: Option<String>,
    meta: bool,
) -> syn::Result<TokenStream2> {
    let ident = item.ident.clone();
    let Fields::Named(named) = &mut item.fields else {
        return Err(syn::Error::new(
            ident.span(),
            "#[item] goes on a struct with named fields",
        ));
    };
    let mut declared = Vec::new();
    for field in named.named.iter_mut() {
        let Some(position) = field.attrs.iter().position(|a| a.path().is_ident("field")) else {
            continue;
        };
        let attr = field.attrs.remove(position);
        let name = field.ident.as_ref().expect("named").to_string();
        declared.push(field_spec(&attr, name, &field.ty)?);
    }
    if let Some(key) = &key {
        if !declared.iter().any(|d| d.schema["key"] == key.as_str()) {
            return Err(syn::Error::new(
                ident.span(),
                format!("key = \"{key}\" names no #[field]"),
            ));
        }
    }
    if !meta {
        let id: syn::FieldsNamed = syn::parse_quote!({
            /// Row id, given by the SDK.
            pub id: ::std::string::String
        });
        named
            .named
            .insert(0, id.named.into_iter().next().expect("one field"));
    }
    let specs: Vec<&TokenStream2> = declared.iter().map(|d| &d.spec).collect();
    let schemas: Vec<&Value> = declared.iter().map(|d| &d.schema).collect();
    let emission = write_emission(
        "item",
        &sanitize_key(&ident.to_string()),
        &json!({ "kind": "item", "name": ident.to_string(), "key": key, "fields": schemas })
            .to_string(),
    );
    let fields_fn = quote! {
        fn fields() -> &'static [::portaki_sdk::resource::FieldSpec] {
            const FIELDS: &[::portaki_sdk::resource::FieldSpec] = &[#(#specs),*];
            FIELDS
        }
    };
    let trait_impl = if meta {
        quote! { impl ::portaki_sdk::resource::Meta for #ident { #fields_fn } }
    } else {
        let key = match &key {
            Some(key) => quote! { ::core::option::Option::Some(#key) },
            None => quote! { ::core::option::Option::None },
        };
        quote! {
            impl ::portaki_sdk::resource::Item for #ident {
                const KEY: ::core::option::Option<&'static str> = #key;
                #fields_fn
                fn id(&self) -> &str { &self.id }
            }
        }
    };
    Ok(quote! {
        #emission
        #[derive(Debug, Clone, Default, ::serde::Serialize, ::serde::Deserialize)]
        #[serde(default)]
        #item
        #trait_impl
    })
}

fn field_spec(attr: &syn::Attribute, key: String, ty: &Type) -> syn::Result<Declared> {
    let mut required = false;
    let mut max_chars: Option<usize> = None;
    let mut between: Option<(f64, f64)> = None;
    let mut label: Option<String> = None;
    let mut description: Option<String> = None;
    let mut forced: Option<String> = None;
    attr.parse_nested_meta(|meta| {
        let name = meta.path.get_ident().map(ToString::to_string).unwrap_or_default();
        match name.as_str() {
            "required" => required = true,
            "max_chars" => max_chars = Some(meta.value()?.parse::<syn::LitInt>()?.base10_parse()?),
            "between" => {
                let content;
                syn::parenthesized!(content in meta.input);
                let list = content.parse_terminated(<syn::Lit as syn::parse::Parse>::parse, syn::Token![,])?;
                let values: Vec<f64> = list.iter().filter_map(lit_number).collect();
                if values.len() != 2 {
                    return Err(meta.error("between(min, max)"));
                }
                between = Some((values[0], values[1]));
            }
            "label" => label = Some(meta.value()?.parse::<syn::LitStr>()?.value()),
            "description" => description = Some(meta.value()?.parse::<syn::LitStr>()?.value()),
            "kind" => forced = Some(meta.value()?.parse::<syn::LitStr>()?.value()),
            _ => {
                return Err(meta.error(
                    "unknown #[field] attribute — required, max_chars, between, label, description, kind",
                ))
            }
        }
        Ok(())
    })?;
    let Some(label) = label else {
        return Err(syn::Error::new(
            attr.span(),
            "#[field] needs label = \"<bundle key>\"",
        ));
    };
    let inner = unwrap_option(ty);
    let (kind, wire_type) = match (forced.as_deref(), type_name(inner).as_deref()) {
        (Some("time_range"), _) => (quote!(TimeRange), "time_range"),
        (Some(other), _) => {
            return Err(syn::Error::new(
                attr.span(),
                format!("unknown kind `{other}` — time_range"),
            ))
        }
        (None, Some("I18nText")) => (quote!(Localized), "localized"),
        (None, Some("String")) => (quote!(Text), "text"),
        (None, Some("bool")) => (quote!(Toggle), "toggle"),
        (None, Some(n)) if NUMBERS.contains(&n) => (quote!(Number), "number"),
        (None, _) => (quote!(Choice), "select"),
    };
    let options = if wire_type == "select" {
        quote! { ::core::option::Option::Some(<#inner as ::portaki_sdk::resource::Choice>::options) }
    } else {
        quote! { ::core::option::Option::None }
    };
    let opt_usize = |v: Option<usize>| match v {
        Some(v) => quote!(::core::option::Option::Some(#v)),
        None => quote!(::core::option::Option::None),
    };
    let opt_f64 = |v: Option<f64>| match v {
        Some(v) => quote!(::core::option::Option::Some(#v)),
        None => quote!(::core::option::Option::None),
    };
    let opt_str = |v: &Option<String>| match v {
        Some(v) => quote!(::core::option::Option::Some(#v)),
        None => quote!(::core::option::Option::None),
    };
    let max_tokens = opt_usize(max_chars);
    let min_tokens = opt_f64(between.map(|b| b.0));
    let maxn_tokens = opt_f64(between.map(|b| b.1));
    let description_tokens = opt_str(&description);
    let spec = quote! {
        ::portaki_sdk::resource::FieldSpec {
            key: #key,
            kind: ::portaki_sdk::resource::FieldKind::#kind,
            label: #label,
            description: #description_tokens,
            required: #required,
            max_chars: #max_tokens,
            min: #min_tokens,
            max: #maxn_tokens,
            options: #options,
        }
    };
    let mut schema = json!({ "key": key, "type": wire_type, "required": required, "label": label });
    if let Some(description) = description {
        schema["description"] = json!(description);
    }
    if let Some(max) = max_chars {
        schema["maxChars"] = json!(max);
    }
    if let Some((min, max)) = between {
        schema["min"] = json!(min);
        schema["max"] = json!(max);
    }
    if wire_type == "select" {
        schema["optionsType"] = json!(type_name(inner).unwrap_or_default());
    }
    Ok(Declared { spec, schema })
}

const NUMBERS: [&str; 12] = [
    "i8", "i16", "i32", "i64", "isize", "u8", "u16", "u32", "u64", "usize", "f32", "f64",
];

fn lit_number(lit: &syn::Lit) -> Option<f64> {
    match lit {
        syn::Lit::Int(i) => i.base10_parse().ok(),
        syn::Lit::Float(f) => f.base10_parse().ok(),
        _ => None,
    }
}

fn unwrap_option(ty: &Type) -> &Type {
    if let Type::Path(path) = ty {
        if let Some(last) = path.path.segments.last() {
            if last.ident == "Option" {
                if let PathArguments::AngleBracketed(args) = &last.arguments {
                    if let Some(GenericArgument::Type(inner)) = args.args.first() {
                        return inner;
                    }
                }
            }
        }
    }
    ty
}

fn type_name(ty: &Type) -> Option<String> {
    match ty {
        Type::Path(path) => path.path.segments.last().map(|s| s.ident.to_string()),
        _ => None,
    }
}

// ── #[collection] ────────────────────────────────────────────────────────────────────────────

/// `#[collection(name = "rules", item = Rule, meta = Display, max = 30, import)]` on a unit struct.
pub fn expand_collection(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = syn::parse_macro_input!(item as ItemStruct);
    let mut name: Option<String> = None;
    let mut row: Option<syn::Path> = None;
    let mut meta: Option<syn::Path> = None;
    let mut max: Option<usize> = None;
    let mut import = false;
    let mut readiness = true;
    let parser = syn::meta::parser(|m| {
        let key = m
            .path
            .get_ident()
            .map(ToString::to_string)
            .unwrap_or_default();
        match key.as_str() {
            "name" => name = Some(m.value()?.parse::<syn::LitStr>()?.value()),
            "item" => row = Some(m.value()?.parse()?),
            "meta" => meta = Some(m.value()?.parse()?),
            "max" => max = Some(m.value()?.parse::<syn::LitInt>()?.base10_parse()?),
            "import" => import = true,
            "readiness" => readiness = m.value()?.parse::<syn::LitBool>()?.value,
            _ => {
                return Err(
                    m.error("#[collection] takes name, item, meta, max, import, readiness = false")
                )
            }
        }
        Ok(())
    });
    syn::parse_macro_input!(attr with parser);
    let (Some(name), Some(row)) = (name, row) else {
        return syn::Error::new(
            Span::call_site(),
            "#[collection] needs name = \"…\" and item = Type",
        )
        .to_compile_error()
        .into();
    };
    let ident = &item.ident;
    let meta_ty = meta.as_ref().map_or_else(|| quote!(()), |m| quote!(#m));
    let max_tokens = match max {
        Some(max) => quote!(::core::option::Option::Some(#max)),
        None => quote!(::core::option::Option::None),
    };
    let snake = sanitize_key(&name).to_lowercase();
    let update = format_ident!("__portaki_{}_update_config", snake);
    let import_fn = format_ident!("__portaki_{}_import_rows", snake);
    let readiness_fn = format_ident!("__portaki_{}_publish_readiness", snake);
    let entity_mod = format_ident!("__portaki_{}_resource_entity", snake);
    let row_name = row
        .segments
        .last()
        .map(|s| s.ident.to_string())
        .unwrap_or_default();
    let meta_name = meta
        .as_ref()
        .and_then(|m| m.segments.last())
        .map(|s| s.ident.to_string());
    let emission = write_emission(
        "resource",
        &sanitize_key(&name),
        &json!({
            "kind": "resource",
            "name": name,
            "resourceKind": "collection",
            "item": row_name,
            "meta": meta_name,
            "max": max,
            "order": "manual",
            "import": if import { json!({ "command": "importRows" }) } else { Value::Null },
        })
        .to_string(),
    );
    let import_tokens = if import {
        quote! {
            #[doc(hidden)]
            #[::portaki_sdk::command(name = "importRows")]
            pub fn #import_fn(
                ctx: ::portaki_sdk::Context,
                args: ::portaki_sdk::resource::ImportArgs,
            ) -> ::portaki_sdk::Result<::portaki_sdk::resource::merge::ImportReport> {
                ::portaki_sdk::resource::import_command::<#row, #meta_ty>(&ctx, #ident::DECL, args)
            }
        }
    } else {
        quote! {}
    };
    let readiness_tokens = if readiness {
        quote! {
            #[doc(hidden)]
            #[::portaki_sdk::query(name = "publishReadiness")]
            pub fn #readiness_fn(
                ctx: ::portaki_sdk::Context,
            ) -> ::portaki_sdk::Result<::portaki_sdk::contracts::publish::PublishReadiness> {
                ::portaki_sdk::resource::readiness::<#row, #meta_ty>(&ctx, #ident::DECL)
            }
        }
    } else {
        quote! {}
    };
    quote! {
        #emission
        #item

        impl #ident {
            /// What `#[collection]` declared.
            pub const DECL: ::portaki_sdk::resource::CollectionDecl =
                ::portaki_sdk::resource::CollectionDecl { name: #name, max: #max_tokens };

            /// The stored collection.
            pub fn load(
                _ctx: &::portaki_sdk::Context,
            ) -> ::portaki_sdk::Result<::portaki_sdk::resource::Collection<#row, #meta_ty>> {
                ::portaki_sdk::resource::store::load(Self::DECL)
            }

            /// Stores the collection.
            pub fn save(
                collection: &::portaki_sdk::resource::Collection<#row, #meta_ty>,
            ) -> ::portaki_sdk::Result<()> {
                ::portaki_sdk::resource::store::save(collection)
            }
        }

        #[doc(hidden)]
        #[::portaki_sdk::command(name = "updateConfig")]
        pub fn #update(
            ctx: ::portaki_sdk::Context,
            args: ::serde_json::Value,
        ) -> ::portaki_sdk::Result<()> {
            ::portaki_sdk::resource::host_command::<#row, #meta_ty>(&ctx, #ident::DECL, &args)
        }

        #import_tokens
        #readiness_tokens

        #[doc(hidden)]
        #[allow(dead_code, missing_docs)]
        mod #entity_mod {
            use ::portaki_sdk::prelude::{DateTime, Utc, Uuid};

            #[::portaki_sdk::entity(schema_version = 1)]
            pub struct PortakiResource {
                pub id: Uuid,
                pub resource: String,
                pub rows_json: String,
                pub meta_json: String,
                pub updated_at: DateTime<Utc>,
            }
        }
    }
    .into()
}
