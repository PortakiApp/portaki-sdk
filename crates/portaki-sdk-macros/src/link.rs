//! `link_module_crate!` — names the module library from one of its integration tests.

use proc_macro::TokenStream;
use quote::{format_ident, quote};

/// Expands to `extern crate <lib> as _;`.
pub fn expand(input: TokenStream) -> TokenStream {
    let explicit = if input.is_empty() {
        None
    } else {
        Some(syn::parse_macro_input!(input as syn::Ident))
    };

    let lib = match explicit {
        Some(ident) => ident,
        None => match library_name() {
            Some(name) => format_ident!("{}", name),
            None => {
                return syn::Error::new(
                    proc_macro2::Span::call_site(),
                    "cannot tell which library this test belongs to (no CARGO_PKG_NAME) — \
                     name it: conformance!(my_module)",
                )
                .to_compile_error()
                .into();
            }
        },
    };

    quote! {
        extern crate #lib as _;
    }
    .into()
}

/// `[lib] name`, else the package name as rustc spells a crate.
fn library_name() -> Option<String> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").ok();
    let from_lib_section = manifest_dir
        .and_then(|dir| std::fs::read_to_string(std::path::Path::new(&dir).join("Cargo.toml")).ok())
        .and_then(|cargo| lib_section_name(&cargo));
    from_lib_section.or_else(|| {
        std::env::var("CARGO_PKG_NAME")
            .ok()
            .map(|name| name.replace('-', "_"))
    })
}

/// `name = "…"` inside `[lib]`, read line by line — no TOML parser for one key.
fn lib_section_name(cargo: &str) -> Option<String> {
    let mut in_lib = false;
    for line in cargo.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_lib = line == "[lib]";
            continue;
        }
        if !in_lib {
            continue;
        }
        if let Some(rest) = line.strip_prefix("name") {
            let value = rest.trim_start().strip_prefix('=')?.trim();
            return value
                .strip_prefix('"')
                .and_then(|value| value.strip_suffix('"'))
                .map(|name| name.replace('-', "_"));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::lib_section_name;

    #[test]
    fn the_lib_section_names_the_crate() {
        let cargo = "[package]\nname = \"access-guide\"\n\n[lib]\nname = \"guide\"\ncrate-type = [\"cdylib\", \"rlib\"]\n";
        assert_eq!(lib_section_name(cargo).as_deref(), Some("guide"));
    }

    #[test]
    fn without_a_lib_name_the_package_decides() {
        let cargo =
            "[package]\nname = \"access-guide\"\n\n[lib]\ncrate-type = [\"cdylib\", \"rlib\"]\n";
        assert_eq!(lib_section_name(cargo), None);
    }
}
