//! `#[wire]` — Portaki gateway JSON DTOs default to camelCase field names.
//!
//! Serde serializes Rust field names as-is (`snake_case`). The Portaki wire
//! format (gateway, shells, SDUI actions, email context) expects camelCase.
//! Module authors mark wire types with this attribute instead of repeating
//! `#[serde(rename_all = "camelCase")]` and common derives.

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{parse_macro_input, parse_quote, Attribute, DeriveInput, Meta, Path, Token};

/// Serde direction for the wire DTO.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WireMode {
    /// Emit both `Serialize` and `Deserialize` when neither is already derived.
    Both,
    /// Prefer `Serialize` only when neither derive is present.
    SerializeOnly,
    /// Prefer `Deserialize` only when neither derive is present.
    DeserializeOnly,
}

/// Parsed `#[wire]` / `#[wire(serialize)]` / `#[wire(no_debug)]` flags.
struct WireOptions {
    mode: WireMode,
    /// Skip injecting `Debug` (types that intentionally omit it).
    no_debug: bool,
    /// Skip injecting `Clone` (full wire adds Clone by default).
    no_clone: bool,
}

impl Parse for WireOptions {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut opts = Self {
            mode: WireMode::Both,
            no_debug: false,
            no_clone: false,
        };

        if input.is_empty() {
            return Ok(opts);
        }

        let idents = Punctuated::<syn::Ident, Token![,]>::parse_terminated(input)?;
        let mut saw_mode = false;

        for ident in idents {
            if ident == "serialize" {
                if saw_mode {
                    return Err(syn::Error::new(
                        ident.span(),
                        "only one of `serialize` / `deserialize` is allowed",
                    ));
                }
                opts.mode = WireMode::SerializeOnly;
                saw_mode = true;
            } else if ident == "deserialize" {
                if saw_mode {
                    return Err(syn::Error::new(
                        ident.span(),
                        "only one of `serialize` / `deserialize` is allowed",
                    ));
                }
                opts.mode = WireMode::DeserializeOnly;
                saw_mode = true;
            } else if ident == "no_debug" {
                opts.no_debug = true;
            } else if ident == "no_clone" {
                opts.no_clone = true;
            } else {
                return Err(syn::Error::new(
                    ident.span(),
                    "expected `serialize`, `deserialize`, `no_debug`, and/or `no_clone`",
                ));
            }
        }

        Ok(opts)
    }
}

/// Expands `#[wire]` on a struct or enum used on the Portaki JSON wire.
pub fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    let opts = parse_macro_input!(attr as WireOptions);
    let mut input = parse_macro_input!(item as DeriveInput);

    ensure_wire_derives(&mut input, &opts);
    ensure_rename_all_camel_case(&mut input.attrs);

    TokenStream::from(quote! { #input })
}

fn ensure_wire_derives(input: &mut DeriveInput, opts: &WireOptions) {
    let flags = derive_flags(&input.attrs);
    let mut paths: Punctuated<Path, Token![,]> = Punctuated::new();

    if !opts.no_debug && !flags.debug {
        paths.push(parse_quote! { Debug });
    }

    // Full wire DTOs almost always need Clone; serialize/deserialize-only rarely do.
    let want_clone = opts.mode == WireMode::Both && !opts.no_clone;
    if want_clone && !flags.clone {
        paths.push(parse_quote! { Clone });
    }

    // When the author already opted into manual serde derives, leave them alone.
    if !flags.serialize && !flags.deserialize {
        match opts.mode {
            WireMode::Both => {
                paths.push(parse_quote! { ::serde::Serialize });
                paths.push(parse_quote! { ::serde::Deserialize });
            }
            WireMode::SerializeOnly => {
                paths.push(parse_quote! { ::serde::Serialize });
            }
            WireMode::DeserializeOnly => {
                paths.push(parse_quote! { ::serde::Deserialize });
            }
        }
    }

    if paths.is_empty() {
        return;
    }

    if let Some(derive_attr) = input
        .attrs
        .iter_mut()
        .find(|attr| attr.path().is_ident("derive"))
    {
        if let Meta::List(list) = &mut derive_attr.meta {
            if !list.tokens.is_empty() {
                list.tokens.extend(quote! { , });
            }
            list.tokens.extend(quote! { #paths });
            return;
        }
    }

    input.attrs.insert(0, parse_quote! { #[derive(#paths)] });
}

#[derive(Default)]
struct DeriveFlags {
    serialize: bool,
    deserialize: bool,
    debug: bool,
    clone: bool,
}

fn derive_flags(attrs: &[Attribute]) -> DeriveFlags {
    let mut flags = DeriveFlags::default();

    for attr in attrs {
        if !attr.path().is_ident("derive") {
            continue;
        }
        let Ok(metas) = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
        else {
            continue;
        };
        for meta in metas {
            let ident = meta
                .path()
                .get_ident()
                .cloned()
                .or_else(|| meta.path().segments.last().map(|s| s.ident.clone()));
            let Some(ident) = ident else {
                continue;
            };
            if ident == "Serialize" {
                flags.serialize = true;
            } else if ident == "Deserialize" {
                flags.deserialize = true;
            } else if ident == "Debug" {
                flags.debug = true;
            } else if ident == "Clone" {
                flags.clone = true;
            }
        }
    }

    flags
}

fn ensure_rename_all_camel_case(attrs: &mut Vec<Attribute>) {
    if has_rename_all(attrs) {
        return;
    }

    let rename: Attribute = parse_quote! { #[serde(rename_all = "camelCase")] };

    // `serde` derive helpers must appear *after* `#[derive(Serialize, Deserialize)]`
    // (`legacy_derive_helpers` / rust-lang#79202).
    if let Some(pos) = attrs
        .iter()
        .rposition(|attr| attr.path().is_ident("derive"))
    {
        attrs.insert(pos + 1, rename);
    } else {
        attrs.push(rename);
    }
}

fn has_rename_all(attrs: &[Attribute]) -> bool {
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        let Ok(metas) = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
        else {
            continue;
        };
        for meta in metas {
            if meta.path().is_ident("rename_all") {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::{
        derive_flags, ensure_rename_all_camel_case, ensure_wire_derives, WireMode, WireOptions,
    };
    use quote::quote;
    use syn::parse_quote;

    fn both() -> WireOptions {
        WireOptions {
            mode: WireMode::Both,
            no_debug: false,
            no_clone: false,
        }
    }

    fn serialize_only() -> WireOptions {
        WireOptions {
            mode: WireMode::SerializeOnly,
            no_debug: false,
            no_clone: false,
        }
    }

    #[test]
    fn detects_common_derives() {
        let input: syn::DeriveInput = parse_quote! {
            #[derive(Debug, Clone, Serialize, Deserialize)]
            struct Foo { a: u8 }
        };
        let flags = derive_flags(&input.attrs);
        assert!(flags.debug && flags.clone && flags.serialize && flags.deserialize);
    }

    #[test]
    fn adds_full_wire_derives_when_missing() {
        let mut input: syn::DeriveInput = parse_quote! {
            struct Foo { a: u8 }
        };
        ensure_wire_derives(&mut input, &both());
        let flags = derive_flags(&input.attrs);
        assert!(flags.debug && flags.clone && flags.serialize && flags.deserialize);
    }

    #[test]
    fn adds_debug_and_serialize_only() {
        let mut input: syn::DeriveInput = parse_quote! {
            struct Foo { a: u8 }
        };
        ensure_wire_derives(&mut input, &serialize_only());
        let flags = derive_flags(&input.attrs);
        assert!(flags.debug && flags.serialize);
        assert!(!flags.clone && !flags.deserialize);
    }

    #[test]
    fn no_debug_skips_debug() {
        let mut input: syn::DeriveInput = parse_quote! {
            struct Foo { a: u8 }
        };
        let mut opts = both();
        opts.no_debug = true;
        ensure_wire_derives(&mut input, &opts);
        let flags = derive_flags(&input.attrs);
        assert!(!flags.debug);
        assert!(flags.clone && flags.serialize && flags.deserialize);
    }

    #[test]
    fn no_clone_skips_clone() {
        let mut input: syn::DeriveInput = parse_quote! {
            struct Foo { a: u8 }
        };
        let mut opts = both();
        opts.no_clone = true;
        ensure_wire_derives(&mut input, &opts);
        let flags = derive_flags(&input.attrs);
        assert!(!flags.clone);
        assert!(flags.debug && flags.serialize && flags.deserialize);
    }

    #[test]
    fn preserves_existing_serde_and_adds_debug() {
        let mut input: syn::DeriveInput = parse_quote! {
            #[derive(Serialize)]
            struct Foo { a: u8 }
        };
        ensure_wire_derives(&mut input, &serialize_only());
        let flags = derive_flags(&input.attrs);
        assert!(flags.debug && flags.serialize);
        assert!(!flags.deserialize && !flags.clone);
    }

    #[test]
    fn keeps_extra_derives() {
        let mut input: syn::DeriveInput = parse_quote! {
            #[derive(Default, PartialEq)]
            struct Foo { a: u8 }
        };
        ensure_wire_derives(&mut input, &both());
        let rendered = quote!(#input).to_string();
        assert!(rendered.contains("Default"));
        assert!(rendered.contains("PartialEq"));
        assert!(rendered.contains("Debug"));
        assert!(rendered.contains("Clone"));
    }

    #[test]
    fn adds_rename_all_when_missing() {
        let mut input: syn::DeriveInput = parse_quote! {
            #[derive(Serialize)]
            struct Foo { a: u8 }
        };
        ensure_rename_all_camel_case(&mut input.attrs);
        let rendered = quote!(#input).to_string();
        assert!(rendered.contains("rename_all"));
        assert!(rendered.contains("camelCase"));
        let derive_pos = rendered.find("derive").expect("derive");
        let rename_pos = rendered.find("rename_all").expect("rename_all");
        assert!(
            derive_pos < rename_pos,
            "rename_all must follow derive: {rendered}"
        );
    }

    #[test]
    fn keeps_existing_rename_all() {
        let mut input: syn::DeriveInput = parse_quote! {
            #[serde(tag = "type", rename_all = "camelCase")]
            #[derive(Serialize)]
            enum Foo { Bar }
        };
        ensure_rename_all_camel_case(&mut input.attrs);
        let rename_count = input
            .attrs
            .iter()
            .filter(|attr| {
                attr.path().is_ident("serde")
                    && attr
                        .parse_args_with(
                            syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
                        )
                        .map(|metas| metas.iter().any(|meta| meta.path().is_ident("rename_all")))
                        .unwrap_or(false)
            })
            .count();
        assert_eq!(rename_count, 1);
    }

    #[test]
    fn parses_combined_flags() {
        let opts: WireOptions = syn::parse_quote! { serialize, no_debug };
        assert_eq!(opts.mode, WireMode::SerializeOnly);
        assert!(opts.no_debug);
        assert!(!opts.no_clone);
    }
}
