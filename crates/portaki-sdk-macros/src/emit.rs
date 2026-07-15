//! Side-effect emission of JSON manifest fragments during proc-macro expansion.
//!
//! Every public macro in this crate calls [`write_emission`] to persist metadata when Cargo sets
//! `OUT_DIR`. The emitted `quote! {}` is empty — authors never see runtime code from emissions.
//!
//! If `OUT_DIR` is unset (e.g. `rust-analyzer` expansion), writes are silently skipped; no error.
//! Filename keys pass through [`sanitize_key`] (non `[A-Za-z0-9_-]` → `_`).

use std::fs;
use std::path::PathBuf;

use proc_macro2::TokenStream;
use quote::quote;

/// Writes one JSON emission file during proc-macro expansion (when `OUT_DIR` is set).
pub fn write_emission(kind: &str, key: &str, json: &str) -> TokenStream {
    if let Ok(out_dir) = std::env::var("OUT_DIR") {
        let dir = PathBuf::from(out_dir).join("portaki-emissions");
        let _ = fs::create_dir_all(&dir);
        let path = dir.join(format!("{kind}-{key}.json"));
        let _ = fs::write(path, json);
    }
    quote! {}
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
