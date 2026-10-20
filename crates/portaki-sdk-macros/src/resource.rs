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
    let mut initial: Option<syn::Ident> = None;
    let mut done: Option<syn::Ident> = None;
    for (index, variant) in item.variants.iter_mut().enumerate() {
        for (marker, slot) in [("initial", &mut initial), ("done", &mut done)] {
            if let Some(position) = variant.attrs.iter().position(|a| a.path().is_ident(marker)) {
                variant.attrs.remove(position);
                *slot = Some(variant.ident.clone());
            }
        }
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
    let lifecycle = match (&initial, &done) {
        (Some(initial), Some(done)) => quote! {
            impl ::portaki_sdk::resource::Lifecycle for #ident {
                const INITIAL: Self = Self::#initial;
                const DONE: Self = Self::#done;
            }
        },
        (None, None) => quote! {},
        _ => {
            return Err(syn::Error::new(
                ident.span(),
                "a lifecycle marks one #[initial] and one #[done] variant",
            ))
        }
    };
    Ok(quote! {
        #lifecycle
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
    let mut max_rows: Option<usize> = None;
    let mut accept: Option<String> = None;
    let mut max_bytes: Option<u32> = None;
    let mut default: Option<String> = None;
    let mut visible_when: Option<String> = None;
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
            "max_rows" => max_rows = Some(meta.value()?.parse::<syn::LitInt>()?.base10_parse()?),
            "accept" => accept = Some(meta.value()?.parse::<syn::LitStr>()?.value()),
            "max_bytes" => max_bytes = Some(meta.value()?.parse::<syn::LitInt>()?.base10_parse()?),
            "visible_when" => visible_when = Some(meta.value()?.parse::<syn::LitStr>()?.value()),
            "default" => {
                default = Some(match meta.value()?.parse::<syn::Lit>()? {
                    syn::Lit::Bool(b) => b.value.to_string(),
                    syn::Lit::Int(i) => i.base10_digits().to_string(),
                    syn::Lit::Float(f) => f.base10_digits().to_string(),
                    // A string is JSON when it reads as JSON (`"[\"email\"]"`), else a text.
                    syn::Lit::Str(text) => serde_json::from_str::<Value>(&text.value())
                        .map_or_else(|_| json!(text.value()).to_string(), |v| v.to_string()),
                    other => return Err(syn::Error::new(other.span(), "default = bool, number or \"…\"")),
                });
            }
            _ => {
                return Err(meta.error(
                    "unknown #[field] attribute — required, max_chars, between, label, description, kind, max_rows, accept, max_bytes, default, visible_when",
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
    // No description declared: the help the bundles already hold for this label, if any.
    let description = description.or_else(|| crate::bundle::conventional_help(&label));
    let placeholder = crate::bundle::conventional_key(&label, "placeholder");
    let inner = unwrap_option(ty);
    let row_type = vec_of(inner);
    let (kind, wire_type) = match (forced.as_deref(), type_name(inner).as_deref()) {
        (Some("time_range"), _) => (quote!(TimeRange), "time_range"),
        (Some("photo"), _) => (quote!(Photo), "photo"),
        // Localized text to an import; the editor and the booklet read it as formatted.
        (Some("rich_text"), Some("I18nText")) => (quote!(RichText), "localized"),
        (Some("url"), Some("String")) => (quote!(Url), "url"),
        (Some("file"), Some("String")) => (quote!(File), "file"),
        (Some("choices"), Some("Vec")) => (quote!(Choices), "choices"),
        (Some(other), _) => {
            return Err(syn::Error::new(
                attr.span(),
                format!("unknown kind `{other}` — time_range, photo, rich_text (on I18nText), url, file (on String)"),
            ))
        }
        (None, Some("Vec")) if row_type.as_ref().and_then(type_name).as_deref() == Some("String") => {
            (quote!(Labels), "labels")
        }
        (None, Some("Vec")) => (quote!(Rows), "structured"),
        (None, Some("I18nText")) => (quote!(Localized), "localized"),
        (None, Some("String")) => (quote!(Text), "text"),
        (None, Some("bool")) => (quote!(Toggle), "toggle"),
        (None, Some(n)) if NUMBERS.contains(&n) => (quote!(Number), "number"),
        (None, _) => (quote!(Choice), "select"),
    };
    let options = if wire_type == "select" {
        quote! { ::core::option::Option::Some(<#inner as ::portaki_sdk::resource::Choice>::options) }
    } else if let ("choices", Some(choice)) = (wire_type, &row_type) {
        quote! { ::core::option::Option::Some(<#choice as ::portaki_sdk::resource::Choice>::options) }
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
    let placeholder_tokens = opt_str(&placeholder);
    let max_rows_tokens = opt_usize(max_rows);
    let required_tokens = match crate::bundle::conventional_key(&label, "required") {
        Some(key) if required => quote! {
            ::core::option::Option::Some(|| ::portaki_sdk::bundle_text!(#key))
        },
        _ => quote!(::core::option::Option::None),
    };
    let too_many_tokens = match crate::bundle::conventional_key(&label, "tooMany") {
        Some(key) if max_rows.is_some() => quote! {
            ::core::option::Option::Some(|| ::portaki_sdk::bundle_text!(#key))
        },
        _ => quote!(::core::option::Option::None),
    };
    let default_tokens = opt_str(&default);
    let visible_tokens = opt_str(&visible_when);
    let accept_tokens = opt_str(&accept);
    let max_bytes_tokens = match max_bytes {
        Some(v) => quote!(::core::option::Option::Some(#v)),
        None => quote!(::core::option::Option::None),
    };
    let rows_tokens = match (wire_type, &row_type) {
        ("structured", Some(row)) => quote! {
            ::core::option::Option::Some(<#row as ::portaki_sdk::resource::Item>::fields)
        },
        _ => quote!(::core::option::Option::None),
    };
    let spec = quote! {
        ::portaki_sdk::resource::FieldSpec {
            key: #key,
            kind: ::portaki_sdk::resource::FieldKind::#kind,
            label: #label,
            description: #description_tokens,
            placeholder: #placeholder_tokens,
            required: #required,
            max_chars: #max_tokens,
            min: #min_tokens,
            max: #maxn_tokens,
            options: #options,
            rows: #rows_tokens,
            max_rows: #max_rows_tokens,
            accept: #accept_tokens,
            max_bytes: #max_bytes_tokens,
            default: #default_tokens,
            visible_when: #visible_tokens,
            required_text: #required_tokens,
            too_many_text: #too_many_tokens,
        }
    };
    let mut schema = json!({ "key": key, "type": wire_type, "required": required, "label": label });
    if let Some(description) = description {
        schema["description"] = json!(description);
    }
    if forced.as_deref() == Some("rich_text") {
        schema["format"] = json!("rich_text");
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
    if wire_type == "choices" {
        schema["optionsType"] = json!(row_type.as_ref().and_then(type_name).unwrap_or_default());
    }
    if wire_type == "structured" {
        schema["itemType"] = json!(row_type.as_ref().and_then(type_name).unwrap_or_default());
    }
    if let Some(max) = max_rows {
        schema["maxItems"] = json!(max);
    }
    if let Some(accept) = &accept {
        schema["accept"] = json!(accept);
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

/// `T` of a `Vec<T>`.
fn vec_of(ty: &Type) -> Option<Type> {
    let Type::Path(path) = ty else { return None };
    let last = path.path.segments.last()?;
    if last.ident != "Vec" {
        return None;
    }
    match &last.arguments {
        PathArguments::AngleBracketed(args) => match args.args.first()? {
            GenericArgument::Type(inner) => Some(inner.clone()),
            _ => None,
        },
        _ => None,
    }
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
    let mut required = false;
    let mut readiness = true;
    let mut check: Option<syn::Path> = None;
    let mut legacy: Option<syn::Path> = None;
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
            "required" => required = true,
            "readiness" => readiness = m.value()?.parse::<syn::LitBool>()?.value,
            "check" => check = Some(m.value()?.parse()?),
            "legacy" => legacy = Some(m.value()?.parse()?),
            _ => return Err(m.error(
                "#[collection] takes name, item, meta, max, import, required, check = path, legacy = path, readiness = false",
            )),
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
    let legacy_tokens = match &legacy {
        Some(path) => quote! {
            ::core::option::Option::Some(|| ::portaki_sdk::resource::store::legacy_json(#path()))
        },
        None => quote!(::core::option::Option::None),
    };
    let with_check = match &check {
        Some(path) => quote!(.map(|c| c.with_check(#path))),
        None => quote!(),
    };
    let meta_ty = meta
        .as_ref()
        .map_or_else(|| quote!(::portaki_sdk::resource::NoMeta), |m| quote!(#m));
    let max_tokens = match max {
        Some(max) => quote!(::core::option::Option::Some(#max)),
        None => quote!(::core::option::Option::None),
    };
    let too_many_key = format!("host.{name}.tooMany");
    let has_too_many = crate::bundle::bundle_files(&std::path::PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default(),
    ))
    .iter()
    .any(|(_, path)| crate::bundle::has_key(path, &too_many_key));
    let too_many_tokens = if max.is_some() && has_too_many {
        quote! { ::core::option::Option::Some(|| ::portaki_sdk::bundle_text!(#too_many_key)) }
    } else {
        quote!(::core::option::Option::None)
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
            "required": required,
            "order": "manual",
            "import": if import { json!({ "command": "importRows" }) } else { Value::Null },
        })
        .to_string(),
    );
    let import_example = format!(r#"{{"resource":"{name}","rows":[]}}"#);
    let import_tokens = if import {
        quote! {
            #[doc(hidden)]
            #[::portaki_sdk::command(name = "importRows", __sdk, example(label = "Importer des lignes", input = #import_example))]
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
            #[::portaki_sdk::query(name = "publishReadiness", __sdk, example(label = "Prêt à publier ?"))]
            pub fn #readiness_fn(
                ctx: ::portaki_sdk::Context,
            ) -> ::portaki_sdk::Result<::portaki_sdk::contracts::publish::PublishReadiness> {
                ::portaki_sdk::resource::readiness(&ctx, &#ident::load(&ctx)?)
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
                ::portaki_sdk::resource::CollectionDecl {
                    name: #name,
                    max: #max_tokens,
                    required: #required,
                    too_many: #too_many_tokens,
                    legacy: #legacy_tokens,
                };

            /// The stored collection.
            pub fn load(
                _ctx: &::portaki_sdk::Context,
            ) -> ::portaki_sdk::Result<::portaki_sdk::resource::Collection<#row, #meta_ty>> {
                ::portaki_sdk::resource::store::load(Self::DECL)#with_check
            }

            /// Stores the collection.
            pub fn save(
                collection: &::portaki_sdk::resource::Collection<#row, #meta_ty>,
            ) -> ::portaki_sdk::Result<()> {
                ::portaki_sdk::resource::store::save(collection)
            }
        }

        #[doc(hidden)]
        #[::portaki_sdk::command(name = "updateConfig", __sdk, example(label = "Enregistrer sans rien changer", input = "{}"))]
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
                pub stay_id: Option<Uuid>,
                pub updated_at: DateTime<Utc>,
            }
        }
    }
    .into()
}

// ── #[record] ────────────────────────────────────────────────────────────────────────────────

/// `#[record(name = "reports", item = Report, status = ReportStatus, due_hours = 24,
/// task_title = "task.title", task_done = "host.detail.resolve", hooks, note_email = "host-reply")]`
/// on a unit struct. `note_email` declares the email `on_note` sends (`email.{id}.description`).
pub fn expand_record(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = syn::parse_macro_input!(item as ItemStruct);
    let mut name: Option<String> = None;
    let mut row: Option<syn::Path> = None;
    let mut status: Option<syn::Path> = None;
    let mut due_hours: Option<i64> = None;
    let mut task_title: Option<String> = None;
    let mut task_done: Option<String> = None;
    let mut hooks = false;
    let mut legacy: Option<syn::Path> = None;
    let mut note_email: Option<String> = None;
    let mut commands = true;
    let parser = syn::meta::parser(|m| {
        let key = m
            .path
            .get_ident()
            .map(ToString::to_string)
            .unwrap_or_default();
        match key.as_str() {
                "name" => name = Some(m.value()?.parse::<syn::LitStr>()?.value()),
                "item" => row = Some(m.value()?.parse()?),
                "status" => status = Some(m.value()?.parse()?),
                "due_hours" => due_hours = Some(m.value()?.parse::<syn::LitInt>()?.base10_parse()?),
                "task_title" => task_title = Some(m.value()?.parse::<syn::LitStr>()?.value()),
                "task_done" => task_done = Some(m.value()?.parse::<syn::LitStr>()?.value()),
                "hooks" => hooks = true,
                "legacy" => legacy = Some(m.value()?.parse()?),
                "note_email" => note_email = Some(m.value()?.parse::<syn::LitStr>()?.value()),
            "commands" => commands = m.value()?.parse::<syn::LitBool>()?.value,
                _ => return Err(m.error(
                    "#[record] takes name, item, status, due_hours, task_title, task_done, hooks, legacy = path, note_email, commands",
                )),
            }
        Ok(())
    });
    syn::parse_macro_input!(attr with parser);
    let (Some(name), Some(row), Some(status)) = (name, row, status) else {
        return syn::Error::new(
            Span::call_site(),
            "#[record] needs name = \"…\", item = Type and status = Lifecycle",
        )
        .to_compile_error()
        .into();
    };
    if due_hours.is_some() && (task_title.is_none() || task_done.is_none()) {
        return syn::Error::new(
            Span::call_site(),
            "due_hours makes timeline tasks: give task_title and task_done (bundle keys)",
        )
        .to_compile_error()
        .into();
    }
    let ident = &item.ident;
    let snake = sanitize_key(&name).to_lowercase();
    let f = |what: &str| format_ident!("__portaki_{}_{}", snake, what);
    let (submit, add, set_status, note, list_for_stay, tasks, toggle, complete, entity_mod) = (
        f("submit"),
        f("add"),
        f("set_status"),
        f("note"),
        f("list_for_stay"),
        f("timeline_tasks"),
        f("task_toggle"),
        f("task_complete"),
        f("resource_entity"),
    );
    // The module's earlier records: each one stored as it comes, its id kept.
    let record_legacy = match &legacy {
        Some(path) => quote! {
            ::core::option::Option::Some(|decl| {
                for entry in #path()? {
                    entry.save(decl)?;
                }
                ::core::result::Result::Ok(())
            })
        },
        None => quote!(::core::option::Option::None),
    };
    let due_tokens = match due_hours {
        Some(h) => quote!(::core::option::Option::Some(#h)),
        None => quote!(::core::option::Option::None),
    };
    let row_name = row
        .segments
        .last()
        .map(|s| s.ident.to_string())
        .unwrap_or_default();
    let emission = write_emission(
        "resource",
        &sanitize_key(&name),
        &json!({
            "kind": "resource", "name": name, "resourceKind": "record",
            "item": row_name, "order": Value::Null, "import": Value::Null,
        })
        .to_string(),
    );
    let hooks_impl = if hooks {
        quote! {}
    } else {
        quote! {
            impl ::portaki_sdk::resource::RecordHooks for #ident {
                type Item = #row;
                type Status = #status;
            }
        }
    };
    let entry = quote!(::portaki_sdk::resource::Entry<#row, #status>);
    // The reply's email, declared on the generated `note` so the manifest lists it.
    let note_email = note_email.map(|id| {
        let description = format!("email.{id}.description");
        quote! {
            #[::portaki_sdk::email(id = #id, audience = ::portaki_sdk::host::email::EmailAudience::Guest,
                requires_guest_email, description_key = #description)]
        }
    });
    let id_example = r#"{"id":"9d8c7b6a-5f4e-4d3c-8b2a-1f0e9d8c7b6a","status":"in_progress"}"#;
    let note_example =
        r#"{"id":"9d8c7b6a-5f4e-4d3c-8b2a-1f0e9d8c7b6a","text":"Le technicien passe demain."}"#;
    let tasks_tokens = match (task_title, task_done) {
        (Some(title), Some(done)) if due_hours.is_some() => quote! {
            #[doc(hidden)]
            #[::portaki_sdk::query(name = "timelineTasks", __sdk, example(label = "Semaine en cours",
                input = r#"{"propertyId":"5f0c2b1e-8a4d-4c6f-9e21-3b7d9a6c4e10","from":"2026-06-15T00:00:00Z","to":"2026-06-22T00:00:00Z","stays":[]}"#))]
            pub fn #tasks(
                ctx: ::portaki_sdk::Context,
                args: ::portaki_sdk::contracts::timeline::TimelineTasksArgs,
            ) -> ::portaki_sdk::Result<::portaki_sdk::contracts::timeline::TimelineTasks> {
                ::portaki_sdk::resource::record::timeline_tasks::<#ident>(
                    &ctx, #ident::DECL, &args, ::portaki_sdk::bundle_text!(#title), ::portaki_sdk::bundle_text!(#done))
            }

            #[doc(hidden)]
            #[::portaki_sdk::command(name = "taskToggle", __sdk, example(label = "Clore",
                input = r#"{"propertyId":"5a1c9e2b-4f3d-4b8a-8e7f-6d2c1b0a9f8e","taskId":"9d8c7b6a-5f4e-4d3c-8b2a-1f0e9d8c7b6a","itemId":"done","done":true}"#))]
            pub fn #toggle(
                _ctx: ::portaki_sdk::Context,
                args: ::portaki_sdk::contracts::timeline::TaskToggleArgs,
            ) -> ::portaki_sdk::Result<()> {
                if args.item_id != "done" {
                    return ::core::result::Result::Err(::portaki_sdk::PortakiError::Host("item_not_found".into()));
                }
                ::portaki_sdk::resource::record::complete_task::<#ident>(#ident::DECL, &args.task_id, args.done)
            }

            #[doc(hidden)]
            #[::portaki_sdk::command(name = "taskComplete", __sdk, example(label = "Clore",
                input = r#"{"propertyId":"5a1c9e2b-4f3d-4b8a-8e7f-6d2c1b0a9f8e","taskId":"9d8c7b6a-5f4e-4d3c-8b2a-1f0e9d8c7b6a"}"#))]
            pub fn #complete(
                _ctx: ::portaki_sdk::Context,
                args: ::portaki_sdk::contracts::timeline::TaskCompleteArgs,
            ) -> ::portaki_sdk::Result<()> {
                ::portaki_sdk::resource::record::complete_task::<#ident>(#ident::DECL, &args.task_id, true)
            }
        },
        _ => quote! {},
    };
    // `commands = false`: typed per-stay storage the module drives itself, no operation of its own.
    let operations = if commands {
        quote! {
        #[doc(hidden)]
        #[::portaki_sdk::command(name = "submit", __sdk, guest, example(label = "Un dépôt", input = "{}"))]
        pub fn #submit(ctx: ::portaki_sdk::Context, args: ::serde_json::Value) -> ::portaki_sdk::Result<()> {
            ::portaki_sdk::resource::record::submit_command::<#ident>(&ctx, #ident::DECL, &args)
        }

        #[doc(hidden)]
        #[::portaki_sdk::command(name = "add", __sdk, example(label = "Ajouté par l'hôte",
            input = r#"{"stayId":"5d1a7e3c-2b9f-4c8d-a6e0-7f3b1c9d2e54"}"#))]
        pub fn #add(ctx: ::portaki_sdk::Context, args: ::serde_json::Value) -> ::portaki_sdk::Result<()> {
            ::portaki_sdk::resource::record::add_command::<#ident>(&ctx, #ident::DECL, &args)
        }

        #[doc(hidden)]
        #[::portaki_sdk::command(name = "setStatus", __sdk, example(label = "Pris en charge", input = #id_example))]
        pub fn #set_status(
            ctx: ::portaki_sdk::Context,
            args: ::portaki_sdk::resource::record::SetStatusArgs,
        ) -> ::portaki_sdk::Result<()> {
            ::portaki_sdk::resource::record::set_status_command::<#ident>(&ctx, #ident::DECL, args)
        }

        #[doc(hidden)]
        #note_email
        #[::portaki_sdk::command(name = "note", __sdk, example(label = "Répondre", input = #note_example))]
        pub fn #note(
            ctx: ::portaki_sdk::Context,
            args: ::portaki_sdk::resource::record::NoteArgs,
        ) -> ::portaki_sdk::Result<()> {
            ::portaki_sdk::resource::record::note_command::<#ident>(&ctx, #ident::DECL, args)
        }

        #[doc(hidden)]
        #[::portaki_sdk::query(name = "listForStay", __sdk, guest, example(label = "Les dépôts du séjour"))]
        pub fn #list_for_stay(
            ctx: ::portaki_sdk::Context,
        ) -> ::portaki_sdk::Result<::std::vec::Vec<::serde_json::Value>> {
            ::portaki_sdk::resource::record::list_for_stay_query::<#ident>(&ctx, #ident::DECL)
        }

        #tasks_tokens

        }
    } else {
        quote! {}
    };
    quote! {
        #emission
        #item
        #hooks_impl

        impl #ident {
            /// What `#[record]` declared.
            pub const DECL: ::portaki_sdk::resource::RecordDecl =
                ::portaki_sdk::resource::RecordDecl { name: #name, due_hours: #due_tokens, legacy: #record_legacy };

            /// Every record, newest first.
            pub fn list(_ctx: &::portaki_sdk::Context) -> ::portaki_sdk::Result<::std::vec::Vec<#entry>> {
                ::portaki_sdk::resource::record::list(Self::DECL)
            }

            /// The records of one stay, newest first.
            pub fn for_stay(
                _ctx: &::portaki_sdk::Context,
                stay: ::portaki_sdk::prelude::Uuid,
            ) -> ::portaki_sdk::Result<::std::vec::Vec<#entry>> {
                ::portaki_sdk::resource::record::for_stay(Self::DECL, stay)
            }

            /// One record.
            pub fn get(
                _ctx: &::portaki_sdk::Context,
                id: ::portaki_sdk::prelude::Uuid,
            ) -> ::portaki_sdk::Result<::core::option::Option<#entry>> {
                ::portaki_sdk::resource::record::get(Self::DECL, id)
            }
        }

        #operations

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
                pub stay_id: Option<Uuid>,
                pub updated_at: DateTime<Utc>,
            }
        }
    }
    .into()
}
