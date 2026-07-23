//! `#[wire]` — Portaki gateway JSON DTOs default to camelCase field names.
//!
//! Serde serializes Rust field names as-is (`snake_case`). The Portaki wire
//! format (gateway, shells, SDUI actions, email context) expects camelCase.
//! Module authors mark wire types with this attribute instead of repeating
//! `#[serde(rename_all = "camelCase")]`.

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{parse_macro_input, parse_quote, Attribute, DeriveInput, Meta, Path, Token};

/// Optional `#[wire]` / `#[wire(serialize)]` / `#[wire(deserialize)]` flags.
enum WireMode {
    /// Emit both `Serialize` and `Deserialize` when neither is already derived.
    Both,
    /// Prefer `Serialize` only when neither derive is present.
    SerializeOnly,
    /// Prefer `Deserialize` only when neither derive is present.
    DeserializeOnly,
}

impl Parse for WireMode {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        if input.is_empty() {
            return Ok(Self::Both);
        }

        let ident: syn::Ident = input.parse()?;
        if ident == "serialize" {
            Ok(Self::SerializeOnly)
        } else if ident == "deserialize" {
            Ok(Self::DeserializeOnly)
        } else {
            Err(syn::Error::new(
                ident.span(),
                "expected empty `#[wire]`, `#[wire(serialize)]`, or `#[wire(deserialize)]`",
            ))
        }
    }
}

/// Expands `#[wire]` on a struct or enum used on the Portaki JSON wire.
pub fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mode = parse_macro_input!(attr as WireMode);
    let mut input = parse_macro_input!(item as DeriveInput);

    ensure_serde_derives(&mut input, mode);
    ensure_rename_all_camel_case(&mut input.attrs);

    TokenStream::from(quote! { #input })
}

fn ensure_serde_derives(input: &mut DeriveInput, mode: WireMode) {
    let (has_serialize, has_deserialize) = serde_derive_flags(&input.attrs);

    if has_serialize || has_deserialize {
        return;
    }

    let paths: Punctuated<Path, Token![,]> = match mode {
        WireMode::Both => {
            parse_quote! { ::serde::Serialize, ::serde::Deserialize }
        }
        WireMode::SerializeOnly => parse_quote! { ::serde::Serialize },
        WireMode::DeserializeOnly => parse_quote! { ::serde::Deserialize },
    };

    if let Some(derive_attr) = input.attrs.iter_mut().find(|attr| attr.path().is_ident("derive"))
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

fn serde_derive_flags(attrs: &[Attribute]) -> (bool, bool) {
    let mut has_serialize = false;
    let mut has_deserialize = false;

    for attr in attrs {
        if !attr.path().is_ident("derive") {
            continue;
        }
        let Ok(metas) = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
        else {
            continue;
        };
        for meta in metas {
            let Some(ident) = meta.path().get_ident() else {
                let segments = &meta.path().segments;
                if let Some(last) = segments.last() {
                    if last.ident == "Serialize" {
                        has_serialize = true;
                    } else if last.ident == "Deserialize" {
                        has_deserialize = true;
                    }
                }
                continue;
            };
            if ident == "Serialize" {
                has_serialize = true;
            } else if ident == "Deserialize" {
                has_deserialize = true;
            }
        }
    }

    (has_serialize, has_deserialize)
}

fn ensure_rename_all_camel_case(attrs: &mut Vec<Attribute>) {
    if has_rename_all(attrs) {
        return;
    }

    let rename: Attribute = parse_quote! { #[serde(rename_all = "camelCase")] };

    // `serde` derive helpers must appear *after* `#[derive(Serialize, Deserialize)]`
    // (`legacy_derive_helpers` / rust-lang#79202).
    if let Some(pos) = attrs.iter().rposition(|attr| attr.path().is_ident("derive")) {
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
    use super::{ensure_rename_all_camel_case, ensure_serde_derives, serde_derive_flags, WireMode};
    use quote::quote;
    use syn::parse_quote;

    #[test]
    fn detects_serde_derives() {
        let input: syn::DeriveInput = parse_quote! {
            #[derive(Debug, Serialize, Deserialize)]
            struct Foo { a: u8 }
        };
        assert_eq!(serde_derive_flags(&input.attrs), (true, true));
    }

    #[test]
    fn adds_both_derives_when_missing() {
        let mut input: syn::DeriveInput = parse_quote! {
            #[derive(Debug, Clone)]
            struct Foo { a: u8 }
        };
        ensure_serde_derives(&mut input, WireMode::Both);
        let (ser, de) = serde_derive_flags(&input.attrs);
        assert!(ser && de);
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
        assert!(derive_pos < rename_pos, "rename_all must follow derive: {rendered}");
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
    fn serialize_only_mode() {
        let mut input: syn::DeriveInput = parse_quote! {
            struct Foo { a: u8 }
        };
        ensure_serde_derives(&mut input, WireMode::SerializeOnly);
        assert_eq!(serde_derive_flags(&input.attrs), (true, false));
    }
}
