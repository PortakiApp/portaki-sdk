//! Side-effect emission of JSON manifest fragments during proc-macro expansion.
//!
//! Every public macro in this crate calls [`write_emission`] to persist metadata when Cargo sets
//! `OUT_DIR`. The emitted `quote! {}` is empty — authors never see runtime code from emissions.
//!
//! If `OUT_DIR` is unset (e.g. `rust-analyzer` expansion), writes are silently skipped; no error.
//! Filename keys pass through [`sanitize_key`] (non `[A-Za-z0-9_-]` → `_`).
//!
//! Every fragment carries a `build` id, the same for all expansions of one compilation. Files
//! are never removed, so a renamed or deleted declaration leaves its old fragment behind; the
//! CLI keeps only the fragments of the latest compilation, told apart by this id.

use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use proc_macro2::TokenStream;
use quote::quote;

/// Writes one JSON emission file during proc-macro expansion (when `OUT_DIR` is set).
pub fn write_emission(kind: &str, key: &str, json: &str) -> TokenStream {
    if let Ok(out_dir) = std::env::var("OUT_DIR") {
        let dir = PathBuf::from(out_dir).join("portaki-emissions");
        let _ = fs::create_dir_all(&dir);
        let path = dir.join(format!("{kind}-{key}.json"));
        let _ = fs::write(path, stamped(json));
    }
    quote! {}
}

/// One id per compilation: rustc loads this proc-macro once per crate it compiles, and re-expands
/// every macro each time it does — incremental compilation does not cache expansions.
fn build_id() -> &'static str {
    static BUILD: OnceLock<String> = OnceLock::new();
    BUILD.get_or_init(|| {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos());
        format!("{nanos}-{}", std::process::id())
    })
}

/// Adds the compilation's `build` id to an emission object.
fn stamped(json: &str) -> String {
    match serde_json::from_str::<serde_json::Value>(json) {
        Ok(serde_json::Value::Object(mut fields)) => {
            fields.insert("build".into(), build_id().into());
            serde_json::Value::Object(fields).to_string()
        }
        _ => json.to_string(),
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
    use super::{sanitize_key, stamped};

    #[test]
    fn sanitize_key_replaces_invalid_chars() {
        assert_eq!(sanitize_key("home.cards"), "home_cards");
    }

    #[test]
    fn emissions_of_one_compilation_share_a_build_id() {
        let a: serde_json::Value = serde_json::from_str(&stamped(r#"{"kind":"query"}"#)).unwrap();
        let b: serde_json::Value = serde_json::from_str(&stamped(r#"{"kind":"nav"}"#)).unwrap();
        assert!(a["build"].is_string());
        assert_eq!(a["build"], b["build"]);
        assert_eq!(a["kind"], "query");
    }
}
