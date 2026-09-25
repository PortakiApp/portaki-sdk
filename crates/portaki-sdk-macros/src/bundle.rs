//! `bundle_text!` expansion — a text of the module's own bundles, every language at once.
//!
//! The bundles are found here, at compile time: every `i18n/*.json` then every
//! `email_i18n/*.json` under the crate root, embedded with `include_str!` (so an edit rebuilds).
//! A module no longer keeps a hand-written `include_str!` list per language, which drifts the day
//! a language is added.

use std::path::{Path, PathBuf};

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Expr, Token};

/// The two folders, in lookup order: a key in both takes the `i18n/` text.
const DIRS: [&str; 2] = ["i18n", "email_i18n"];

struct BundleText {
    key: Expr,
    vars: Option<Expr>,
}

impl Parse for BundleText {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let key = input.parse()?;
        let vars = if input.parse::<Option<Token![,]>>()?.is_some() && !input.is_empty() {
            Some(input.parse()?)
        } else {
            None
        };
        input.parse::<Option<Token![,]>>()?;
        Ok(Self { key, vars })
    }
}

/// Expands `bundle_text!(key)` / `bundle_text!(key, &[("name", value)])`.
pub fn expand(input: TokenStream) -> TokenStream {
    let parsed = syn::parse_macro_input!(input as BundleText);
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default());
    match expand_in(&root, parsed) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn expand_in(root: &Path, parsed: BundleText) -> syn::Result<TokenStream2> {
    let span = proc_macro2::Span::call_site();
    let bundles = bundle_files(root);
    if bundles.is_empty() {
        return Err(syn::Error::new(
            span,
            format!(
                "bundle_text! found no i18n/*.json nor email_i18n/*.json under {}",
                root.display()
            ),
        ));
    }
    // A literal key is checked now: a typo would otherwise show the key to the reader.
    if let Expr::Lit(syn::ExprLit {
        lit: syn::Lit::Str(key),
        ..
    }) = &parsed.key
    {
        if !bundles.iter().any(|(_, path)| has_key(path, &key.value())) {
            return Err(syn::Error::new(
                key.span(),
                format!("`{}` is in no i18n/ nor email_i18n/ bundle", key.value()),
            ));
        }
    }
    let entries = bundles.iter().map(|(lang, path)| {
        let path = path.to_string_lossy();
        quote! { (#lang, include_str!(#path)) }
    });
    let key = &parsed.key;
    let vars = parsed
        .vars
        .as_ref()
        .map_or_else(|| quote! { &[] }, |vars| quote! { #vars });
    Ok(quote! {{
        static PORTAKI_BUNDLES: &[(&str, &str)] = &[#(#entries),*];
        ::portaki_sdk::contracts::i18n::I18nText::from_bundles(PORTAKI_BUNDLES, #key, #vars)
    }})
}

/// `(language, path)` of every bundle, `i18n/` first, each folder sorted by file name. The
/// language is the file stem's first part: `fr-FR.json` and `fr.json` are both `fr`.
fn bundle_files(root: &Path) -> Vec<(String, PathBuf)> {
    let mut found = Vec::new();
    for dir in DIRS {
        let mut files: Vec<PathBuf> = std::fs::read_dir(root.join(dir))
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
            .collect();
        files.sort();
        for path in files {
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            let lang = stem.split(['-', '_']).next().unwrap_or_default();
            if !lang.is_empty() {
                found.push((lang.to_ascii_lowercase(), path));
            }
        }
    }
    found
}

fn has_key(path: &Path, key: &str) -> bool {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .is_some_and(|bundle| bundle.get(key).is_some_and(serde_json::Value::is_string))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn module() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        for (dir, file, json) in [
            ("i18n", "fr-FR.json", r#"{"a":"Bonjour"}"#),
            ("i18n", "en-US.json", r#"{"a":"Hello"}"#),
            ("email_i18n", "de.json", r#"{"b":"Hallo {name}"}"#),
            ("email_i18n", "notes.txt", "ignored"),
        ] {
            std::fs::create_dir_all(root.path().join(dir)).unwrap();
            std::fs::write(root.path().join(dir).join(file), json).unwrap();
        }
        root
    }

    #[test]
    fn bundles_are_found_i18n_first_with_their_language() {
        let root = module();
        let langs: Vec<String> = bundle_files(root.path())
            .into_iter()
            .map(|(lang, _)| lang)
            .collect();
        assert_eq!(langs, ["en", "fr", "de"]);
    }

    #[test]
    fn a_literal_key_must_exist_somewhere() {
        let root = module();
        let expand = |input: &str| expand_in(root.path(), syn::parse_str(input).unwrap());
        let tokens = expand(r#""b", &[("name", "Ada")]"#).unwrap().to_string();
        assert!(tokens.contains("include_str !"), "{tokens}");
        assert!(tokens.contains("from_bundles"), "{tokens}");
        assert!(expand(r#""a""#).is_ok());
        assert!(
            expand("key_at_runtime").is_ok(),
            "only literals are checked"
        );
        let missing = expand(r#""nope""#).unwrap_err().to_string();
        assert!(missing.contains("no i18n/"), "{missing}");

        let empty = tempfile::tempdir().unwrap();
        assert!(expand_in(empty.path(), syn::parse_str(r#""a""#).unwrap()).is_err());
    }
}
