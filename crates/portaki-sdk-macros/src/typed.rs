//! Typed manifest arguments — `placement = HostPlacement::PropertyStatsCard`, never a string.
//!
//! A proc-macro sees tokens, not values: it cannot read the enum. So it does two things with the
//! path it is given. It emits `Vocabulary::Variant` into the manifest fragment, which
//! `portaki build` turns into the wire string through `portaki_sdk::vocab::wire_of`. And it emits
//! a `const` naming the variant, so the compiler — not a list copied here — says whether it exists.

use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote_spanned};
use syn::parse::ParseStream;
use syn::spanned::Spanned;
use syn::LitStr;

/// A vocabulary a macro argument takes, and where it lives in `portaki_sdk`.
#[derive(Clone, Copy)]
pub(crate) struct Vocab {
    pub name: &'static str,
    module: &'static str,
}

pub(crate) const HOST_PLACEMENT: Vocab = Vocab::of("HostPlacement");
pub(crate) const GUEST_ROLE: Vocab = Vocab::of("GuestRole");
pub(crate) const MATURITY: Vocab = Vocab::of("Maturity");
pub(crate) const MODULE_AUDIENCE: Vocab = Vocab::of("ModuleAudience");
pub(crate) const MODULE_TYPE: Vocab = Vocab::of("ModuleType");
pub(crate) const EMAIL_TRIGGER: Vocab = Vocab::of("EmailTrigger");
pub(crate) const SKIP_WHEN: Vocab = Vocab::of("SkipWhen");
pub(crate) const HOST_FRAGMENT: Vocab = Vocab::of("HostFragmentId");
pub(crate) const DESIGN_ID: Vocab = Vocab::of("DesignId");
pub(crate) const ICON_NAME: Vocab = Vocab::of("IconName");
pub(crate) const EMAIL_AUDIENCE: Vocab = Vocab {
    name: "EmailAudience",
    module: "host::email",
};

impl Vocab {
    const fn of(name: &'static str) -> Self {
        Vocab {
            name,
            module: "vocab",
        }
    }
}

/// One typed argument: what goes in the manifest fragment, and the compile-time check.
pub(crate) struct Typed {
    /// `Vocabulary::Variant`, resolved to the wire string by `portaki build`.
    pub emitted: String,
    pub check: TokenStream2,
    /// `::portaki_sdk::<module>::Vocabulary::Variant`, for code that uses the value.
    pub path: TokenStream2,
}

/// Parses `Vocabulary::Variant` (or the bare `Variant`) for argument `key`.
pub(crate) fn parse(input: ParseStream<'_>, key: &str, vocab: Vocab) -> syn::Result<Typed> {
    if input.peek(LitStr) {
        let literal: LitStr = input.parse()?;
        return Err(syn::Error::new(
            literal.span(),
            format!(
                "`{key}` takes a {name}, not a string — write `{name}::{suggestion}`",
                name = vocab.name,
                suggestion = pascal(&literal.value()),
            ),
        ));
    }
    let path: syn::Path = input.parse()?;
    let segments: Vec<_> = path.segments.iter().collect();
    let variant = &segments.last().expect("a path has a segment").ident;
    if let Some(owner) = segments.len().checked_sub(2).map(|i| &segments[i].ident) {
        if owner != vocab.name {
            return Err(syn::Error::new(
                path.span(),
                format!(
                    "`{key}` takes a {name} — write `{name}::{variant}`",
                    name = vocab.name
                ),
            ));
        }
    }
    let ty = format_ident!("{}", vocab.name);
    let module: TokenStream2 = vocab
        .module
        .split("::")
        .map(|segment| {
            let ident = format_ident!("{}", segment);
            quote::quote!(::#ident)
        })
        .collect();
    let path = quote_spanned! {variant.span()=> ::portaki_sdk #module :: #ty :: #variant };
    let check = quote_spanned! {variant.span()=>
        const _: ::portaki_sdk #module :: #ty = #path;
    };
    Ok(Typed {
        emitted: format!("{}::{}", vocab.name, variant),
        check,
        path,
    })
}

/// `property-stats-card` → `PropertyStatsCard`, for the suggestion of an error.
fn pascal(wire: &str) -> String {
    wire.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            chars
                .next()
                .map(|first| first.to_ascii_uppercase().to_string() + chars.as_str())
                .unwrap_or_default()
        })
        .collect()
}

/// The value of a navigation key — shared by `#[surface]` and `#[nav]`.
///
/// Typed keys take their vocabulary; `path`, `label_key` and `display_name_key` stay strings
/// (a route, and i18n keys that `portaki build` checks against the bundles).
pub(crate) fn nav_value(
    input: ParseStream<'_>,
    key: &str,
    checks: &mut Vec<TokenStream2>,
) -> syn::Result<serde_json::Value> {
    let vocab = match key {
        "placement" => HOST_PLACEMENT,
        "design_id" => DESIGN_ID,
        "icon" => ICON_NAME,
        "role" => GUEST_ROLE,
        "embeds" => HOST_FRAGMENT,
        _ => return Ok(input.parse::<LitStr>()?.value().into()),
    };
    let typed = parse(input, key, vocab)?;
    checks.push(typed.check);
    Ok(typed.emitted.into())
}

/// A wall-clock time `HH:MM`, checked at compile time.
pub(crate) fn local_time(literal: &LitStr) -> syn::Result<String> {
    let value = literal.value();
    let valid = matches!(value.split_once(':'), Some((h, m))
        if h.len() == 2 && m.len() == 2
            && h.parse::<u8>().is_ok_and(|h| h < 24)
            && m.parse::<u8>().is_ok_and(|m| m < 60));
    if valid {
        Ok(value)
    } else {
        Err(syn::Error::new(
            literal.span(),
            "expected a time as \"HH:MM\", e.g. \"10:00\"",
        ))
    }
}

/// An error for the whole attribute, when no token is to blame.
pub(crate) fn error(message: impl std::fmt::Display) -> syn::Error {
    syn::Error::new(Span::call_site(), message)
}

#[cfg(test)]
mod tests {
    use super::{local_time, parse, pascal, HOST_PLACEMENT};
    use syn::parse::Parser;

    fn typed(tokens: &str) -> syn::Result<String> {
        let parser = |input: syn::parse::ParseStream<'_>| parse(input, "placement", HOST_PLACEMENT);
        parser.parse_str(tokens).map(|typed| typed.emitted)
    }

    #[test]
    fn a_path_or_a_bare_variant_is_accepted() {
        assert_eq!(
            typed("HostPlacement::StayAction").unwrap(),
            "HostPlacement::StayAction"
        );
        assert_eq!(
            typed("portaki_sdk::vocab::HostPlacement::StayAction").unwrap(),
            "HostPlacement::StayAction"
        );
        assert_eq!(typed("StayAction").unwrap(), "HostPlacement::StayAction");
    }

    #[test]
    fn a_string_or_another_enum_is_refused_with_the_way_to_write_it() {
        let string = typed(r#""stay-action""#).unwrap_err().to_string();
        assert!(string.contains("HostPlacement::StayAction"), "{string}");
        let other = typed("GuestRole::Card").unwrap_err().to_string();
        assert!(other.contains("HostPlacement::Card"), "{other}");
        assert_eq!(pascal("regulatory.police-form"), "RegulatoryPoliceForm");
    }

    #[test]
    fn a_local_time_is_hours_and_minutes() {
        let lit = |s: &str| syn::LitStr::new(s, proc_macro2::Span::call_site());
        assert_eq!(local_time(&lit("09:30")).unwrap(), "09:30");
        for bad in ["9:30", "24:00", "10:60", "10h00"] {
            assert!(local_time(&lit(bad)).is_err(), "{bad}");
        }
    }
}
