//! Side-effect emission of JSON manifest fragments during proc-macro expansion.
//!
//! Every public macro in this crate calls [`write_emission`] to persist metadata under the
//! crate's `OUT_DIR`. The emitted `quote! {}` is empty — authors never see runtime code from
//! emissions.
//!
//! Cargo sets `OUT_DIR` only for a crate with a build script. Without one, the directory is the
//! one a build script would have had: `<profile>/build/<package>-<hash>/out`, derived from the
//! `--out-dir …/deps` and `-C extra-filename=-<hash>` (or `-C metadata=<hash>`) Cargo passes to
//! rustc — where `portaki build` looks. A module needs no `build.rs`. Outside a Cargo compilation (`rust-analyzer`
//! expansion, rustdoc), writes are silently skipped; no error.
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
    if let Some(out_dir) = out_dir() {
        let dir = out_dir.join("portaki-emissions");
        let _ = fs::create_dir_all(&dir);
        let path = dir.join(format!("{kind}-{key}.json"));
        let _ = fs::write(path, stamped(json));
    }
    quote! {}
}

/// `OUT_DIR`, or the directory Cargo would have given a build script.
fn out_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("OUT_DIR") {
        return Some(PathBuf::from(dir));
    }
    let package = std::env::var("CARGO_PKG_NAME").ok()?;
    derived_out_dir(std::env::args(), &package)
}

/// `<profile>/build/<package>-<hash>/out` from rustc's arguments: `--out-dir <profile>/deps` and
/// `-C extra-filename=-<hash>` (a cdylib: `-C metadata=<hash>`), passed by Cargo. `None` for
/// anything else.
fn derived_out_dir(args: impl Iterator<Item = String>, package: &str) -> Option<PathBuf> {
    let args: Vec<String> = args.collect();
    let value = |flag: &str, prefix: &str| {
        args.iter().enumerate().find_map(|(index, arg)| {
            let inline = arg
                .strip_prefix(flag)
                .map(|rest| rest.trim_start_matches('='));
            let value = match inline {
                Some("") => args.get(index + 1).map(String::as_str),
                other => other,
            }?;
            value.strip_prefix(prefix)
        })
    };
    let deps = PathBuf::from(value("--out-dir", "")?);
    if deps.file_name()? != "deps" {
        return None;
    }
    // A cdylib has no extra-filename; its metadata hash serves the same purpose.
    let hash = value("-C", "extra-filename=-").or_else(|| value("-C", "metadata="))?;
    if hash.is_empty() || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Some(
        deps.parent()?
            .join("build")
            .join(format!("{package}-{hash}"))
            .join("out"),
    )
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
    use super::{derived_out_dir, sanitize_key, stamped};
    use std::path::PathBuf;

    fn args(line: &str) -> impl Iterator<Item = String> + '_ {
        line.split(' ').map(str::to_string)
    }

    /// No build script: the directory is the one Cargo would have given it.
    #[test]
    fn the_out_dir_is_derived_from_rustc_arguments() {
        let rustc = "rustc --crate-name ical_sync --edition=2021 src/lib.rs -C metadata=58e5 \
                     -C extra-filename=-58e578d3c53c15cd --out-dir /m/target/wasm32-unknown-unknown/debug/deps";
        assert_eq!(
            derived_out_dir(args(rustc), "ical-sync"),
            Some(PathBuf::from(
                "/m/target/wasm32-unknown-unknown/debug/build/ical-sync-58e578d3c53c15cd/out"
            ))
        );
        let cdylib = "rustc --crate-type cdylib -C metadata=9c1cc0f0 --out-dir /t/debug/deps";
        assert_eq!(
            derived_out_dir(args(cdylib), "x"),
            Some(PathBuf::from("/t/debug/build/x-9c1cc0f0/out"))
        );
        let inline = "rustc -Cextra-filename=-ab12 --out-dir=/t/debug/deps";
        assert_eq!(
            derived_out_dir(args(inline), "x"),
            Some(PathBuf::from("/t/debug/build/x-ab12/out"))
        );
        for other in [
            "rustdoc --out-dir /t/doc -C extra-filename=-ab12",
            "rustc --out-dir /t/debug/deps",
            "rust-analyzer-proc-macro-srv",
        ] {
            assert_eq!(derived_out_dir(args(other), "x"), None, "{other}");
        }
    }

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
