//! `params` expansion — the shape of an operation's arguments, for tooling.
//!
//! A `#[query]` or `#[command]` handler only names its argument type; the fields live on the
//! struct, which the operation macro cannot see. `#[params]` on that struct emits them, and
//! `portaki build` joins the two into `manifest.queries[]` / `manifest.commands[]`, where the
//! sandbox reads them to offer a form instead of a blank JSON box.
//!
//! The shape is read the way serde reads the struct: `rename_all`, `rename`, `default`, `skip`
//! and `flatten` are honoured, `Option<T>` is optional. An attribute this parser does not know
//! is ignored — describing arguments must never fail a module's build.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use serde_json::{json, Map, Value};
use syn::meta::ParseNestedMeta;
use syn::{
    Attribute, Fields, GenericArgument, Item, ItemEnum, ItemStruct, PathArguments, Token, Type,
};

use crate::emit::{sanitize_key, write_emission};

/// Expands `#[params]` on a struct or an enum.
pub fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    if !attr.is_empty() {
        return syn::Error::new(
            proc_macro2::Span::call_site(),
            "#[portaki_sdk::params] takes no arguments",
        )
        .to_compile_error()
        .into();
    }
    let parsed = syn::parse_macro_input!(item as Item);
    let (name, shape) = match &parsed {
        Item::Struct(item) => (item.ident.to_string(), struct_shape(item)),
        Item::Enum(item) => (item.ident.to_string(), enum_shape(item)),
        other => {
            return syn::Error::new_spanned(
                other,
                "#[portaki_sdk::params] goes on the struct (or enum) a query or command takes",
            )
            .to_compile_error()
            .into();
        }
    };

    let mut emission = Map::new();
    emission.insert("kind".into(), json!("params"));
    emission.insert("name".into(), json!(name));
    if let Value::Object(shape) = shape {
        emission.extend(shape);
    }
    let json = serde_json::to_string_pretty(&Value::Object(emission)).unwrap();
    let emission = write_emission("params", &sanitize_key(&name), &json);
    let output: TokenStream2 = quote! {
        #emission
        #parsed
    };
    output.into()
}

/// What serde says about an item or a field — the part of it that changes the wire shape.
#[derive(Default)]
struct Serde {
    rename: Option<String>,
    rename_all: Option<String>,
    default: bool,
    skip: bool,
    flatten: bool,
}

fn serde_of(attrs: &[Attribute]) -> Serde {
    let mut serde = Serde::default();
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("serde")) {
        // Une erreur ici ne remonte pas : un attribut serde exotique vaut mieux décrit à moitié
        // qu'un module qui ne compile plus.
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename") {
                serde.rename = string_value(&meta)?;
            } else if meta.path.is_ident("rename_all") {
                serde.rename_all = string_value(&meta)?;
            } else if meta.path.is_ident("default") {
                serde.default = true;
                skip_value(&meta)?;
            } else if meta.path.is_ident("skip") || meta.path.is_ident("skip_deserializing") {
                serde.skip = true;
            } else if meta.path.is_ident("flatten") {
                serde.flatten = true;
            } else {
                skip_value(&meta)?;
            }
            Ok(())
        });
    }
    serde
}

/// `rename = "x"` gives the name; `rename(deserialize = "x")` gives the one the handler reads.
fn string_value(meta: &ParseNestedMeta<'_>) -> syn::Result<Option<String>> {
    if meta.input.peek(Token![=]) {
        let value: syn::LitStr = meta.value()?.parse()?;
        return Ok(Some(value.value()));
    }
    let mut found = None;
    meta.parse_nested_meta(|inner| {
        if inner.path.is_ident("deserialize") {
            let value: syn::LitStr = inner.value()?.parse()?;
            found = Some(value.value());
        } else {
            skip_value(&inner)?;
        }
        Ok(())
    })?;
    Ok(found)
}

fn skip_value(meta: &ParseNestedMeta<'_>) -> syn::Result<()> {
    if meta.input.peek(Token![=]) {
        meta.value()?.parse::<syn::Expr>()?;
    } else if meta.input.peek(syn::token::Paren) {
        let content;
        syn::parenthesized!(content in meta.input);
        content.parse::<TokenStream2>()?;
    }
    Ok(())
}

fn doc_of(attrs: &[Attribute]) -> Option<String> {
    let lines: Vec<String> = attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .filter_map(|attr| match &attr.meta {
            syn::Meta::NameValue(pair) => match &pair.value {
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(text),
                    ..
                }) => Some(text.value().trim().to_string()),
                _ => None,
            },
            _ => None,
        })
        .collect();
    let joined = lines
        .split(|line| line.is_empty())
        .next()
        .map(|paragraph| paragraph.join(" "))
        .unwrap_or_default();
    (!joined.is_empty()).then_some(joined)
}

fn struct_shape(item: &ItemStruct) -> Value {
    let container = serde_of(&item.attrs);
    let mut shape = Map::new();
    if let Some(doc) = doc_of(&item.attrs) {
        shape.insert("doc".into(), json!(doc));
    }
    let fields = match &item.fields {
        Fields::Named(named) => named
            .named
            .iter()
            .filter_map(|field| {
                let serde = serde_of(&field.attrs);
                if serde.skip {
                    return None;
                }
                let ident = field.ident.as_ref()?.to_string();
                let ident = ident.trim_start_matches("r#");
                let name = serde
                    .rename
                    .clone()
                    .unwrap_or_else(|| rename_field(ident, container.rename_all.as_deref()));
                let (ty, optional) = type_shape(&field.ty);
                let mut entry = Map::new();
                entry.insert("name".into(), json!(name));
                if let Value::Object(ty) = ty {
                    entry.extend(ty);
                }
                entry.insert(
                    "required".into(),
                    json!(!optional && !serde.default && !container.default),
                );
                if serde.flatten {
                    entry.insert("flatten".into(), json!(true));
                }
                if let Some(doc) = doc_of(&field.attrs) {
                    entry.insert("doc".into(), json!(doc));
                }
                Some(Value::Object(entry))
            })
            .collect(),
        // Un tuple struct n'a pas de noms à proposer : l'éditeur JSON reste la seule saisie juste.
        Fields::Unnamed(_) => {
            shape.insert("opaque".into(), json!(true));
            Vec::new()
        }
        Fields::Unit => Vec::new(),
    };
    shape.insert("fields".into(), Value::Array(fields));
    Value::Object(shape)
}

fn enum_shape(item: &ItemEnum) -> Value {
    let container = serde_of(&item.attrs);
    let mut shape = Map::new();
    if let Some(doc) = doc_of(&item.attrs) {
        shape.insert("doc".into(), json!(doc));
    }
    // Seules les variantes unitaires se choisissent dans une liste ; une variante qui porte des
    // données dépend du marquage serde, que la sandbox laisse à l'éditeur JSON.
    if item
        .variants
        .iter()
        .any(|variant| !matches!(variant.fields, Fields::Unit))
    {
        shape.insert("opaque".into(), json!(true));
        return Value::Object(shape);
    }
    let values: Vec<Value> = item
        .variants
        .iter()
        .filter_map(|variant| {
            let serde = serde_of(&variant.attrs);
            if serde.skip {
                return None;
            }
            Some(json!(serde.rename.unwrap_or_else(|| rename_variant(
                &variant.ident.to_string(),
                container.rename_all.as_deref()
            ))))
        })
        .collect();
    shape.insert("enum".into(), Value::Array(values));
    Value::Object(shape)
}

/// The wire type of a Rust type, and whether it may be left out (`Option<T>`).
fn type_shape(ty: &Type) -> (Value, bool) {
    match ty {
        Type::Reference(reference) => type_shape(&reference.elem),
        Type::Paren(inner) => type_shape(&inner.elem),
        Type::Group(inner) => type_shape(&inner.elem),
        Type::Array(array) => (array_of(&array.elem), false),
        Type::Slice(slice) => (array_of(&slice.elem), false),
        Type::Path(path) => {
            let Some(last) = path.path.segments.last() else {
                return (json!({ "type": "json" }), false);
            };
            let ident = last.ident.to_string();
            let generics: Vec<&Type> = match &last.arguments {
                PathArguments::AngleBracketed(args) => args
                    .args
                    .iter()
                    .filter_map(|arg| match arg {
                        GenericArgument::Type(ty) => Some(ty),
                        _ => None,
                    })
                    .collect(),
                _ => Vec::new(),
            };
            match (ident.as_str(), generics.as_slice()) {
                ("Option", [inner]) => (type_shape(inner).0, true),
                ("Box" | "Rc" | "Arc" | "Cow", [inner]) => type_shape(inner),
                ("Vec" | "VecDeque" | "HashSet" | "BTreeSet" | "IndexSet", [inner]) => {
                    (array_of(inner), false)
                }
                ("HashMap" | "BTreeMap" | "IndexMap", [_, values]) => (
                    json!({ "type": "map", "values": type_shape(values).0 }),
                    false,
                ),
                _ => (scalar(&ident), false),
            }
        }
        _ => (json!({ "type": "json" }), false),
    }
}

fn array_of(inner: &Type) -> Value {
    json!({ "type": "array", "items": type_shape(inner).0 })
}

fn scalar(ident: &str) -> Value {
    match ident {
        "String" | "str" | "char" => json!({ "type": "string" }),
        "bool" => json!({ "type": "boolean" }),
        "i8" | "i16" | "i32" | "i64" | "i128" | "isize" | "u8" | "u16" | "u32" | "u64" | "u128"
        | "usize" => json!({ "type": "integer" }),
        "f32" | "f64" => json!({ "type": "number" }),
        "Uuid" => json!({ "type": "string", "format": "uuid" }),
        "DateTime" | "NaiveDateTime" | "OffsetDateTime" | "PrimitiveDateTime" => {
            json!({ "type": "string", "format": "date-time" })
        }
        "NaiveDate" | "Date" => json!({ "type": "string", "format": "date" }),
        "NaiveTime" | "Time" => json!({ "type": "string", "format": "time" }),
        "Value" => json!({ "type": "json" }),
        // Un type du module : sa forme vient de son propre #[params], s'il en porte un.
        other => json!({ "type": "ref", "ref": other }),
    }
}

/// A field name as serde writes it — fields are `snake_case` in Rust.
fn rename_field(ident: &str, rule: Option<&str>) -> String {
    let words: Vec<&str> = ident.split('_').filter(|word| !word.is_empty()).collect();
    match rule {
        Some("camelCase") => camel(&words, false),
        Some("PascalCase") => camel(&words, true),
        Some("kebab-case") => ident.replace('_', "-"),
        Some("SCREAMING_SNAKE_CASE") => ident.to_uppercase(),
        Some("SCREAMING-KEBAB-CASE") => ident.to_uppercase().replace('_', "-"),
        Some("UPPERCASE") => ident.to_uppercase(),
        _ => ident.to_string(),
    }
}

/// A variant name as serde writes it — variants are `PascalCase` in Rust.
fn rename_variant(ident: &str, rule: Option<&str>) -> String {
    let snake = {
        let mut out = String::new();
        for (index, ch) in ident.chars().enumerate() {
            if ch.is_uppercase() && index > 0 {
                out.push('_');
            }
            out.extend(ch.to_lowercase());
        }
        out
    };
    match rule {
        Some("lowercase") => ident.to_lowercase(),
        Some("UPPERCASE") => ident.to_uppercase(),
        Some("camelCase") => {
            let mut chars = ident.chars();
            chars
                .next()
                .map(|first| first.to_lowercase().chain(chars).collect())
                .unwrap_or_default()
        }
        Some("snake_case") => snake,
        Some("SCREAMING_SNAKE_CASE") => snake.to_uppercase(),
        Some("kebab-case") => snake.replace('_', "-"),
        Some("SCREAMING-KEBAB-CASE") => snake.to_uppercase().replace('_', "-"),
        _ => ident.to_string(),
    }
}

fn camel(words: &[&str], upper_first: bool) -> String {
    words
        .iter()
        .enumerate()
        .map(|(index, word)| {
            if index == 0 && !upper_first {
                return word.to_string();
            }
            let mut chars = word.chars();
            chars
                .next()
                .map(|first| first.to_uppercase().chain(chars).collect::<String>())
                .unwrap_or_default()
        })
        .collect()
}

/// The last path segment of the argument type a handler takes, if it takes one.
///
/// `args: UpdateConfigArgs` → `UpdateConfigArgs`; a reference or a `crate::` path reads the same.
/// Joined by `portaki build` with the `#[params]` emission of that name.
pub fn args_type_name(function_item: &syn::ItemFn) -> Option<String> {
    let syn::FnArg::Typed(second) = function_item.sig.inputs.iter().nth(1)? else {
        return None;
    };
    let mut ty = second.ty.as_ref();
    while let Type::Reference(reference) = ty {
        ty = &reference.elem;
    }
    match ty {
        Type::Path(path) => path.path.segments.last().map(|last| last.ident.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    /// ical-sync's argument struct, as written.
    #[test]
    fn a_struct_is_described_field_by_field() {
        let item: ItemStruct = parse_quote! {
            /// What the host sheet saves.
            #[derive(Deserialize)]
            pub struct UpdateConfigArgs {
                /// Dynamic list from host StepList.
                #[serde(default)]
                pub calendars: Vec<CalendarInput>,
                pub ical_url_primary: String,
                pub retries: Option<u32>,
            }
        };

        assert_eq!(
            struct_shape(&item),
            json!({
                "doc": "What the host sheet saves.",
                "fields": [
                    {
                        "name": "calendars",
                        "type": "array",
                        "items": { "type": "ref", "ref": "CalendarInput" },
                        "required": false,
                        "doc": "Dynamic list from host StepList."
                    },
                    { "name": "ical_url_primary", "type": "string", "required": true },
                    { "name": "retries", "type": "integer", "required": false }
                ]
            })
        );
    }

    /// The names the handler reads, not the Rust ones.
    #[test]
    fn serde_renames_are_honoured() {
        let item: ItemStruct = parse_quote! {
            #[serde(rename_all = "camelCase", default)]
            struct Args {
                stay_id: Uuid,
                #[serde(rename = "from")]
                starts_at: DateTime<Utc>,
                #[serde(skip)]
                cache: String,
                #[serde(with = "something", deserialize_with = "other")]
                raw_payload: serde_json::Value,
            }
        };

        let shape = struct_shape(&item);
        let fields = shape["fields"].as_array().unwrap();

        assert_eq!(fields.len(), 3);
        assert_eq!(
            fields[0],
            json!({ "name": "stayId", "type": "string", "format": "uuid", "required": false })
        );
        assert_eq!(fields[1]["name"], "from");
        assert_eq!(fields[1]["format"], "date-time");
        assert_eq!(
            fields[2],
            json!({ "name": "rawPayload", "type": "json", "required": false })
        );
    }

    #[test]
    fn a_unit_enum_is_a_list_of_values() {
        let item: ItemEnum = parse_quote! {
            #[serde(rename_all = "snake_case")]
            enum CalendarFormat { Ical, GoogleCalendar, #[serde(rename = "json")] JsonFeed }
        };

        assert_eq!(
            enum_shape(&item),
            json!({ "enum": ["ical", "google_calendar", "json"] })
        );
    }

    /// Variants carrying data depend on serde's tagging: the sandbox keeps the JSON editor.
    #[test]
    fn a_data_enum_is_opaque() {
        let item: ItemEnum = parse_quote! { enum Target { Stay(Uuid), All } };

        assert_eq!(enum_shape(&item), json!({ "opaque": true }));
    }

    #[test]
    fn maps_and_nested_generics_are_read() {
        let (shape, optional) =
            type_shape(&parse_quote! { Option<Vec<std::collections::BTreeMap<String, bool>>> });

        assert!(optional);
        assert_eq!(
            shape,
            json!({ "type": "array", "items": { "type": "map", "values": { "type": "boolean" } } })
        );
    }

    #[test]
    fn the_argument_type_of_a_handler_is_named() {
        let with_args: syn::ItemFn = parse_quote! { fn update(ctx: Context, args: crate::args::UpdateConfigArgs) -> Result<()> {} };
        let by_reference: syn::ItemFn = parse_quote! { fn update(ctx: Context, args: &Args) {} };
        let without: syn::ItemFn = parse_quote! { fn refresh(ctx: Context) -> Result<()> {} };

        assert_eq!(
            args_type_name(&with_args).as_deref(),
            Some("UpdateConfigArgs")
        );
        assert_eq!(args_type_name(&by_reference).as_deref(), Some("Args"));
        assert_eq!(args_type_name(&without), None);
    }

    #[test]
    fn only_the_first_doc_paragraph_is_kept() {
        let item: ItemStruct = parse_quote! {
            /// Saves the calendars.
            /// All of them.
            ///
            /// Legacy flat fields are converted.
            struct Args {}
        };

        assert_eq!(
            struct_shape(&item)["doc"],
            "Saves the calendars. All of them."
        );
    }
}
