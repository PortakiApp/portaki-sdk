//! Writes manifest emission fragments to `OUT_DIR/portaki-emissions/`.

use proc_macro2::TokenStream;
use quote::quote;
use syn::LitStr;

/// Generates a compile-time call that writes one JSON emission file.
pub fn write_emission(kind: &str, key: &str, json_expr: TokenStream) -> TokenStream {
    let kind_lit = LitStr::new(kind, proc_macro2::Span::call_site());
    let key_lit = LitStr::new(key, proc_macro2::Span::call_site());

    quote! {
        const _: () = {
            #[allow(unused_imports)]
            use ::std::fs;
            #[allow(unused_imports)]
            use ::std::path::PathBuf;
            let out_dir = match ::std::env::var("OUT_DIR") {
                Ok(value) => value,
                Err(_) => return,
            };
            let dir = PathBuf::from(out_dir).join("portaki-emissions");
            let _ = fs::create_dir_all(&dir);
            let payload = #json_expr;
            let path = dir.join(concat!(#kind_lit, "-", #key_lit, ".json"));
            let _ = fs::write(path, payload);
        };
    }
}

/// Sanitizes a string for use as a filename fragment.
pub fn sanitize_key(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::sanitize_key;

    #[test]
    fn sanitize_key_replaces_invalid_chars() {
        assert_eq!(sanitize_key("home.cards"), "home_cards");
    }
}
