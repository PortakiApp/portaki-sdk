//! `config` expansion — the module's host settings, declared once on a serde struct.
//!
//! The struct is the schema: each `#[field(…)]` becomes an entry of `config.fields` in the
//! catalogue, which the platform validates, stores and hands back in `context.moduleConfig`.
//! Fields without `#[field]` stay out of the schema — the host never edits them.
//!
//! Three things are generated next to the struct:
//!
//! - `load(&Context)`, reading `moduleConfig` through `portaki_sdk::config::load`;
//! - the host query `legacyConfig`, the KV `config` blob the platform imports once — raw, or
//!   through `legacy = path::to::fn` when the old blob does not have the declared keys;
//! - the host command `legacyConfigAdopted`, which the platform calls once the import is stored:
//!   it deletes the KV `config` and the keys of `legacy_keys = ["…"]`;
//! - `#[serde(default)]` on the struct, when it does not carry it: the platform stores only
//!   what the host filled in, so a key may be missing.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use serde_json::{json, Value};
use syn::spanned::Spanned;
use syn::{Fields, GenericArgument, ItemFn, ItemStruct, PathArguments, Type};

use crate::emit::{sanitize_key, write_emission};
use crate::params::{rename_field, serde_of};

/// The operation the platform calls to import a config kept in KV before it owned it.
const LEGACY_QUERY: &str = "legacyConfig";

/// The command the platform calls once that import is stored, to clear the old KV keys.
const ADOPTED_COMMAND: &str = "legacyConfigAdopted";

/// The `type` values of `configField` in `module.v1.json`.
const KINDS: [&str; 10] = [
    "text",
    "textarea",
    "url",
    "number",
    "secret",
    "toggle",
    "select",
    "readonly",
    "structured",
    "localized",
];

/// What `#[portaki_sdk::config(…)]` takes.
#[derive(Default)]
struct ConfigAttrs {
    /// Maps the old KV blob onto the declared keys.
    legacy: Option<syn::Path>,
    /// KV keys besides `config` the old code kept, deleted once the platform holds the config.
    legacy_keys: Vec<String>,
}

/// Expands `#[portaki_sdk::config]` / `#[portaki_sdk::config(legacy = path::to::fn,
/// legacy_keys = ["…"])]` on a struct with named fields.
pub fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut attrs = ConfigAttrs::default();
    let parser =
        syn::meta::parser(|meta| {
            if meta.path.is_ident("legacy") {
                attrs.legacy = Some(meta.value()?.parse()?);
                Ok(())
            } else if meta.path.is_ident("legacy_keys") {
                let list: syn::ExprArray = meta.value()?.parse()?;
                for element in &list.elems {
                    match element {
                        syn::Expr::Lit(syn::ExprLit {
                            lit: syn::Lit::Str(key),
                            ..
                        }) if !key.value().trim().is_empty() => attrs.legacy_keys.push(key.value()),
                        other => return Err(syn::Error::new(
                            other.span(),
                            "legacy_keys are KV keys: legacy_keys = [\"texts/fr\", \"texts/en\"]",
                        )),
                    }
                }
                Ok(())
            } else {
                Err(meta.error(
                    "#[portaki_sdk::config] takes legacy = <fn(Value) -> Value> and \
                 legacy_keys = [\"…\"] — flags go on each field: #[field(required, label = \"…\")]",
                ))
            }
        });
    syn::parse_macro_input!(attr with parser);
    let mut item = syn::parse_macro_input!(item as ItemStruct);
    match expand_struct(&mut item, &attrs) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// `legacy`: maps the raw KV blob onto the declared keys, for `legacyConfig` and for `load`
/// before the platform holds the config.
fn expand_struct(item: &mut ItemStruct, attrs: &ConfigAttrs) -> syn::Result<TokenStream2> {
    let legacy = attrs.legacy.as_ref();
    let fields = declared_fields(item)?;
    if !serde_of(&item.attrs).default {
        item.attrs.push(syn::parse_quote!(#[serde(default)]));
    }

    let name = item.ident.to_string();
    let fields_json = serde_json::to_string(&fields).unwrap();
    let schema = json!({ "kind": "config", "name": name, "fields": fields });
    let config_emission = write_emission(
        "config",
        &sanitize_key(&name),
        &serde_json::to_string_pretty(&schema).unwrap(),
    );

    let map = match legacy {
        Some(path) => quote! { #path },
        None => quote! { ::core::convert::identity },
    };
    let legacy: ItemFn = syn::parse_quote! {
        fn portaki_legacy_config(
            _ctx: ::portaki_sdk::context::Context,
        ) -> ::portaki_sdk::error::Result<::serde_json::Value> {
            ::portaki_sdk::config::legacy_config_mapped(#map)
        }
    };
    let legacy_fn = legacy.sig.ident.to_string();
    let query_emission = write_emission(
        "query",
        &sanitize_key(LEGACY_QUERY),
        &serde_json::to_string_pretty(&json!({
            "kind": "query",
            "name": LEGACY_QUERY,
            "fn": legacy_fn,
            "guest": false,
        }))
        .unwrap(),
    );
    let registration = crate::wasm_handler::register_query(LEGACY_QUERY, &legacy_fn, &legacy);

    let keys = &attrs.legacy_keys;
    let adopted: ItemFn = syn::parse_quote! {
        fn portaki_legacy_config_adopted(
            ctx: ::portaki_sdk::context::Context,
        ) -> ::portaki_sdk::error::Result<::serde_json::Value> {
            ::portaki_sdk::config::legacy_config_adopted(&ctx, &[#(#keys),*])
        }
    };
    let adopted_fn = adopted.sig.ident.to_string();
    let adopted_emission = write_emission(
        "command",
        &sanitize_key(ADOPTED_COMMAND),
        &serde_json::to_string_pretty(&json!({
            "kind": "command",
            "name": ADOPTED_COMMAND,
            "fn": adopted_fn,
            "guest": false,
        }))
        .unwrap(),
    );
    let adopted_registration =
        crate::wasm_handler::register_command(ADOPTED_COMMAND, &adopted_fn, &adopted);

    let ident = &item.ident;
    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();
    Ok(quote! {
        #config_emission
        #query_emission
        #adopted_emission
        #item

        impl #impl_generics #ident #ty_generics #where_clause {
            /// The configuration the platform stores for this install (`context.moduleConfig`).
            ///
            /// When the runtime sends no `moduleConfig` at all (the platform does not hold the
            /// config yet), the KV key `config` is read instead; an empty `moduleConfig` is an
            /// empty config, never the KV. A missing key takes its `Default` value; a
            /// config that does not deserialize is an error, never a silent default.
            pub fn load(ctx: &::portaki_sdk::context::Context) -> ::portaki_sdk::error::Result<Self> {
                ::portaki_sdk::config::load_mapped(ctx, #map)
            }
        }

        #legacy
        #registration
        #adopted
        #adopted_registration

        #[cfg(not(target_arch = "wasm32"))]
        ::portaki_sdk::inventory::submit! {
            ::portaki_sdk::config::ConfigDeclaration { fields: #fields_json }
        }
    })
}

/// Every `#[field]` of the struct as a `configField`, in source order; the attributes are removed.
fn declared_fields(item: &mut ItemStruct) -> syn::Result<Vec<Value>> {
    let container = serde_of(&item.attrs);
    let Fields::Named(named) = &mut item.fields else {
        return Err(syn::Error::new(
            item.ident.span(),
            "#[portaki_sdk::config] goes on a struct with named fields",
        ));
    };

    let mut declared = Vec::new();
    for field in named.named.iter_mut() {
        let Some(position) = field.attrs.iter().position(|a| a.path().is_ident("field")) else {
            continue;
        };
        let attr = field.attrs.remove(position);
        if field.attrs.iter().any(|a| a.path().is_ident("field")) {
            return Err(syn::Error::new(attr.span(), "one #[field(…)] per field"));
        }
        let serde = serde_of(&field.attrs);
        if serde.skip {
            return Err(syn::Error::new(
                attr.span(),
                "a #[serde(skip)] field is never stored — it cannot be declared",
            ));
        }
        let ident = field.ident.as_ref().expect("named field").to_string();
        let key = serde.rename.unwrap_or_else(|| {
            rename_field(
                ident.trim_start_matches("r#"),
                container.rename_all.as_deref(),
            )
        });
        declared.push(field_schema(&attr, key, &field.ty)?);
    }
    Ok(declared)
}

/// One `configField`: `key`, `type`, `required`, `recommended`, `label`, and `description` /
/// `options` when given. Labels are i18n keys; `portaki build` translates them.
fn field_schema(attr: &syn::Attribute, key: String, ty: &Type) -> syn::Result<Value> {
    let mut required = false;
    let mut recommended = false;
    let mut kind: Option<String> = None;
    let mut label: Option<String> = None;
    let mut description: Option<String> = None;
    let mut options: Option<Vec<String>> = None;
    let mut item_id: Option<String> = None;

    let mut set_kind = |value: String, span| -> syn::Result<()> {
        if let Some(previous) = &kind {
            return Err(syn::Error::new(
                span,
                format!("a field has one kind — already `{previous}`"),
            ));
        }
        kind = Some(value);
        Ok(())
    };

    attr.parse_nested_meta(|meta| {
        let span = meta.path.span();
        let name = meta
            .path
            .get_ident()
            .map(ToString::to_string)
            .unwrap_or_default();
        match name.as_str() {
            "required" => required = true,
            "recommended" => recommended = true,
            "secret" | "structured" => set_kind(name, span)?,
            "kind" => {
                let value: syn::LitStr = meta.value()?.parse()?;
                if !KINDS.contains(&value.value().as_str()) {
                    return Err(syn::Error::new(
                        value.span(),
                        format!("unknown kind — one of {}", KINDS.join(", ")),
                    ));
                }
                set_kind(value.value(), value.span())?;
            }
            "label" => label = Some(meta.value()?.parse::<syn::LitStr>()?.value()),
            "description" => description = Some(meta.value()?.parse::<syn::LitStr>()?.value()),
            "item_id" => item_id = Some(meta.value()?.parse::<syn::LitStr>()?.value()),
            "options" => {
                let list: syn::ExprArray = meta.value()?.parse()?;
                let mut values = Vec::new();
                for element in &list.elems {
                    match element {
                        syn::Expr::Lit(syn::ExprLit {
                            lit: syn::Lit::Str(value),
                            ..
                        }) => values.push(value.value()),
                        other => {
                            return Err(syn::Error::new(
                                other.span(),
                                "options are string literals: options = [\"wpa2\", \"wep\"]",
                            ))
                        }
                    }
                }
                options = Some(values);
            }
            _ => {
                return Err(meta.error(
                    "unknown #[field] attribute — required, recommended, secret, structured, \
                     label, description, kind, options, item_id",
                ))
            }
        }
        Ok(())
    })?;

    let Some(label) = label else {
        return Err(syn::Error::new(
            attr.span(),
            "#[field] needs label = \"<i18n key>\" — the dashboard shows it, in every language",
        ));
    };
    if required && recommended {
        return Err(syn::Error::new(
            attr.span(),
            "required already blocks publication — drop recommended",
        ));
    }
    let inferred = inferred_kind(ty);
    // The platform merges a `localized` value language by language: any other kind would have it
    // replace the whole text with the host's language, and a String cannot read the stored object.
    match kind.as_deref() {
        Some(forced) if (forced == "localized") != (inferred == "localized") => {
            return Err(syn::Error::new(
                attr.span(),
                "an I18nText field is `localized`, and only an I18nText field is",
            ))
        }
        _ => {}
    }
    let kind = kind.unwrap_or_else(|| inferred.to_string());
    if item_id.is_some() && kind != "structured" {
        return Err(syn::Error::new(
            attr.span(),
            "item_id names the sub-key identifying a row — only on a structured field",
        ));
    }
    if kind == "readonly" && (required || recommended) {
        return Err(syn::Error::new(
            attr.span(),
            "a readonly field is never required or recommended",
        ));
    }
    match (&options, kind == "select") {
        (None, true) => {
            return Err(syn::Error::new(
                attr.span(),
                "a select field needs options = [\"…\", …]",
            ))
        }
        (Some(_), false) => {
            return Err(syn::Error::new(
                attr.span(),
                "options only go with kind = \"select\"",
            ))
        }
        _ => {}
    }

    let mut schema = json!({
        "key": key,
        "type": kind,
        "required": required,
        "recommended": recommended,
        "label": label,
    });
    if let Some(description) = description {
        schema["description"] = json!(description);
    }
    if let Some(options) = options {
        schema["options"] = json!(options);
    }
    if let Some(id) = item_id {
        schema["item"] = json!({ "id": id });
    }
    // The macro cannot see the row type's fields; `portaki build` reads them from its `#[params]`
    // emission and fills `item` (see `portaki_sdk::config::resolve_items`).
    if kind == "structured" {
        if let Some(row) = row_type(ty) {
            schema["itemType"] = json!(row);
        }
    }
    Ok(schema)
}

/// The module's own type a structured field holds — `Vec<Step>`, `Option<Step>`, `Step` → `Step`;
/// `None` for maps, JSON values and the SDK's types.
fn row_type(ty: &Type) -> Option<String> {
    let Type::Path(path) = ty else {
        return None;
    };
    let last = path.path.segments.last()?;
    match &last.arguments {
        PathArguments::AngleBracketed(args)
            if ["Option", "Box", "Vec"].contains(&last.ident.to_string().as_str()) =>
        {
            match args.args.first()? {
                GenericArgument::Type(inner) => row_type(inner),
                _ => None,
            }
        }
        PathArguments::None if inferred_kind(ty) == "structured" && last.ident != "Value" => {
            Some(last.ident.to_string())
        }
        _ => None,
    }
}

/// `I18nText` → localized, `String` → text, `bool` → toggle, numbers → number, anything else
/// (lists, maps, the module's own types) → structured. `Option<T>` and `Box<T>` read as `T`.
fn inferred_kind(ty: &Type) -> &'static str {
    match ty {
        Type::Reference(reference) => inferred_kind(&reference.elem),
        Type::Paren(inner) => inferred_kind(&inner.elem),
        Type::Group(inner) => inferred_kind(&inner.elem),
        Type::Path(path) => {
            let Some(last) = path.path.segments.last() else {
                return "structured";
            };
            if let PathArguments::AngleBracketed(args) = &last.arguments {
                if last.ident == "Option" || last.ident == "Box" {
                    if let Some(GenericArgument::Type(inner)) = args.args.first() {
                        return inferred_kind(inner);
                    }
                }
            }
            match last.ident.to_string().as_str() {
                "I18nText" => "localized",
                "String" | "str" | "char" => "text",
                "bool" => "toggle",
                "i8" | "i16" | "i32" | "i64" | "i128" | "isize" | "u8" | "u16" | "u32" | "u64"
                | "u128" | "usize" | "f32" | "f64" => "number",
                _ => "structured",
            }
        }
        _ => "structured",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    fn schema_of(mut item: ItemStruct) -> syn::Result<Vec<Value>> {
        declared_fields(&mut item)
    }

    /// The contract's own example, field by field.
    #[test]
    fn the_struct_is_the_schema() {
        let fields = schema_of(parse_quote! {
            pub struct Config {
                #[field(required, label = "config.ssid")]
                pub ssid: String,
                #[field(secret, recommended, label = "config.password", description = "config.password.help")]
                pub password: String,
                #[field(structured, label = "config.contacts")]
                pub contacts: Vec<Contact>,
                #[field(kind = "select", options = ["wpa2", "wep"], label = "config.security")]
                pub security: String,
                #[field(label = "config.guests")]
                pub max_guests: Option<u32>,
                #[field(label = "config.enabled")]
                pub enabled: bool,
                pub hidden: bool,
            }
        })
        .unwrap();

        assert_eq!(
            fields,
            vec![
                json!({ "key": "ssid", "type": "text", "required": true, "recommended": false, "label": "config.ssid" }),
                json!({ "key": "password", "type": "secret", "required": false, "recommended": true,
                        "label": "config.password", "description": "config.password.help" }),
                json!({ "key": "contacts", "type": "structured", "required": false, "recommended": false,
                        "label": "config.contacts", "itemType": "Contact" }),
                json!({ "key": "security", "type": "select", "required": false, "recommended": false,
                        "label": "config.security", "options": ["wpa2", "wep"] }),
                json!({ "key": "max_guests", "type": "number", "required": false, "recommended": false, "label": "config.guests" }),
                json!({ "key": "enabled", "type": "toggle", "required": false, "recommended": false, "label": "config.enabled" }),
            ]
        );
    }

    #[test]
    fn keys_are_the_serde_names() {
        let fields = schema_of(parse_quote! {
            #[serde(rename_all = "camelCase")]
            struct Config {
                #[field(label = "a")]
                wifi_name: String,
                #[field(label = "b")]
                #[serde(rename = "code")]
                door_code: String,
            }
        })
        .unwrap();
        assert_eq!(fields[0]["key"], "wifiName");
        assert_eq!(fields[1]["key"], "code");
    }

    #[test]
    fn a_kind_can_be_forced_and_everything_else_is_structured() {
        let fields = schema_of(parse_quote! {
            struct Config {
                #[field(kind = "url", label = "a")] link: String,
                #[field(kind = "textarea", label = "b")] notes: String,
                #[field(label = "c")] opening: std::collections::BTreeMap<String, String>,
                #[field(label = "d")] kind: Mode,
            }
        })
        .unwrap();
        let kinds: Vec<&str> = fields.iter().map(|f| f["type"].as_str().unwrap()).collect();
        assert_eq!(kinds, ["url", "textarea", "structured", "structured"]);
    }

    #[test]
    fn translated_text_is_localized_and_rows_name_their_type() {
        let fields = schema_of(parse_quote! {
            struct Config {
                #[field(label = "a")] welcome: I18nText,
                #[field(label = "b")] note: Option<portaki_sdk::contracts::i18n::I18nText>,
                #[field(label = "c")] steps: Vec<Step>,
                #[field(label = "d", item_id = "slug")] spots: Option<Vec<Spot>>,
                #[field(label = "e")] hours: Hours,
                #[field(label = "f")] tags: Vec<String>,
                #[field(label = "g")] raw: serde_json::Value,
                #[field(label = "h")] by_day: std::collections::BTreeMap<String, Hours>,
            }
        })
        .unwrap();
        let shown: Vec<Value> = fields
            .iter()
            .map(|f| json!([f["type"], f.get("itemType"), f.get("item")]))
            .collect();
        assert_eq!(
            shown,
            vec![
                json!(["localized", null, null]),
                json!(["localized", null, null]),
                json!(["structured", "Step", null]),
                json!(["structured", "Spot", { "id": "slug" }]),
                json!(["structured", "Hours", null]),
                json!(["structured", null, null]),
                json!(["structured", null, null]),
                json!(["structured", null, null]),
            ]
        );
    }

    #[test]
    fn wrong_declarations_do_not_compile() {
        for item in [
            parse_quote! { struct C { #[field(required)] a: String } },
            parse_quote! { struct C { #[field(kind = "email", label = "a")] a: String } },
            parse_quote! { struct C { #[field(secret, structured, label = "a")] a: String } },
            parse_quote! { struct C { #[field(required, recommended, label = "a")] a: String } },
            parse_quote! { struct C { #[field(kind = "select", label = "a")] a: String } },
            parse_quote! { struct C { #[field(options = ["x"], label = "a")] a: String } },
            parse_quote! { struct C { #[field(kind = "readonly", required, label = "a")] a: String } },
            parse_quote! { struct C { #[field(label = "a", colour = "red")] a: String } },
            parse_quote! { struct C { #[field(label = "a")] #[serde(skip)] a: String } },
            parse_quote! { struct C(String); },
            parse_quote! { struct C { #[field(kind = "localized", label = "a")] a: String } },
            parse_quote! { struct C { #[field(kind = "textarea", label = "a")] a: I18nText } },
            parse_quote! { struct C { #[field(item_id = "id", label = "a")] a: String } },
        ] {
            let item: ItemStruct = item;
            let shown = quote!(#item).to_string();
            assert!(schema_of(item).is_err(), "{shown}");
        }
    }

    #[test]
    fn the_expansion_strips_field_adds_serde_default_load_and_legacy_config() {
        let mut item: ItemStruct = parse_quote! {
            #[derive(Default, Serialize, Deserialize)]
            pub struct Config {
                #[field(required, label = "config.ssid")]
                pub ssid: String,
            }
        };
        let expanded = expand_struct(&mut item, &ConfigAttrs::default())
            .unwrap()
            .to_string();

        assert!(!expanded.contains("field ("), "{expanded}");
        assert!(expanded.contains("# [serde (default)]"), "{expanded}");
        assert!(expanded.contains("pub fn load"), "{expanded}");
        assert!(expanded.contains("\"legacyConfig\""), "{expanded}");
        assert!(expanded.contains("legacy_config_mapped (:: core :: convert :: identity)"));
        let attrs = ConfigAttrs {
            legacy: Some(parse_quote!(crate::old::read)),
            legacy_keys: vec!["texts/fr".into(), "texts/en".into()],
        };
        let mapped = expand_struct(&mut item.clone(), &attrs)
            .unwrap()
            .to_string();
        assert!(
            mapped.contains("legacy_config_mapped (crate :: old :: read)"),
            "{mapped}"
        );
        assert!(
            mapped.contains("load_mapped (ctx , crate :: old :: read)"),
            "{mapped}"
        );
        assert!(expanded.contains("ConfigDeclaration"), "{expanded}");
        assert!(expanded.contains("\"legacyConfigAdopted\""), "{expanded}");
        assert!(
            expanded.contains("legacy_config_adopted (& ctx , & [])"),
            "{expanded}"
        );
        assert!(
            mapped.contains("legacy_config_adopted (& ctx , & [\"texts/fr\" , \"texts/en\"])"),
            "{mapped}"
        );
    }

    #[test]
    fn the_kinds_are_the_schema_ones() {
        let schema: Value =
            serde_json::from_str(include_str!("../../../schema/module.v1.json")).unwrap();
        assert_eq!(
            schema["$defs"]["configField"]["properties"]["type"]["enum"],
            json!(KINDS)
        );
    }

    #[test]
    fn an_existing_serde_default_is_not_repeated() {
        let mut item: ItemStruct = parse_quote! {
            #[serde(default)]
            struct Config { #[field(label = "a")] a: String }
        };
        let expanded = expand_struct(&mut item, &ConfigAttrs::default())
            .unwrap()
            .to_string();
        assert_eq!(expanded.matches("serde (default)").count(), 1, "{expanded}");
    }
}
