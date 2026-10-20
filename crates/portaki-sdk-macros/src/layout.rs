//! Where a declaration lives, checked at compile time in an SDK 11 module: one surface or
//! responsibility per file, the file named after it.
//!
//! | Declaration | File |
//! |---|---|
//! | `#[surface(host, id = "main")]` | `src/host/settings.rs` |
//! | `#[surface(host \| guest, id = "home.card")]` | `src/{side}/home_card.rs` (a leading `host.` / `guest.` is dropped: `guest.form` → `form.rs`) |
//! | `#[command]` / `#[query]` `name = "applyTemplate"` | `src/commands/apply_template.rs` |
//! | `#[connector(builtin)]` / `#[custom_connector(id)]` | `src/connectors/{id}.rs` |
//! | `#[email_vars]` / `#[email_blocks]` | `src/email/vars.rs` / `src/email/blocks.rs` |
//!
//! Two declarations of the same kind cannot share a file: they would need the same name. An
//! `#[email]` rides on its command and lives with it.
//!
//! The rule binds the crate being compiled when its own `Cargo.toml` asks `portaki-sdk` for
//! `unstable` — read per crate, not through Cargo's feature unification, so a workspace that also
//! builds SDK 10 modules leaves them alone. Files outside a `src/` (tests, examples) are not
//! module code. Operations the SDK derives (`#[collection]`, `#[record]`, `#[config]`)
//! carry the `__sdk` flag and live where their declaration is; `portaki check` refuses that flag
//! in a module's own sources.

use std::path::{Path, PathBuf};

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

/// What a macro declares.
pub(crate) enum Declared<'a> {
    Surface { context: &'a str, id: &'a str },
    Operation(&'a str),
    Connector(&'a str),
    EmailVars,
    EmailBlocks,
}

impl Declared<'_> {
    /// The control the message names, as `portaki check` does.
    fn control(&self) -> &'static str {
        match self {
            Declared::Surface { .. } => "one-surface-per-file",
            Declared::Operation(_) => "one-command-per-file",
            Declared::Connector(_) => "one-connector-per-file",
            Declared::EmailVars | Declared::EmailBlocks => "one-email-per-file",
        }
    }

    fn what(&self) -> String {
        match self {
            Declared::Surface { context, id } => format!("{context} surface `{id}`"),
            Declared::Operation(name) => format!("operation `{name}`"),
            Declared::Connector(id) => format!("connector `{id}`"),
            Declared::EmailVars => "#[email_vars]".into(),
            Declared::EmailBlocks => "#[email_blocks]".into(),
        }
    }

    /// The file it belongs in, relative to `src/`.
    pub(crate) fn expected(&self) -> String {
        match self {
            Declared::Surface { context, id } => {
                let id = id
                    .strip_prefix("host.")
                    .or_else(|| id.strip_prefix("guest."))
                    .unwrap_or(id);
                let stem = if *context == "host" && id == "main" {
                    "settings".to_string()
                } else {
                    snake(id)
                };
                format!("{context}/{stem}.rs")
            }
            Declared::Operation(name) => format!("commands/{}.rs", snake(name)),
            Declared::Connector(id) => format!("connectors/{}.rs", snake(id)),
            Declared::EmailVars => "email/vars.rs".into(),
            Declared::EmailBlocks => "email/blocks.rs".into(),
        }
    }
}

/// `applyTemplate`, `home.card`, `open-weather` → `apply_template`, `home_card`, `open_weather`.
pub(crate) fn snake(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_uppercase() {
            if !out.is_empty() && !out.ends_with('_') {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        } else if c == '.' || c == '-' {
            out.push('_');
        } else {
            out.push(c);
        }
    }
    out
}

/// In an SDK 11 module, a `const` that fails to compile when `file!()` is not the file the
/// declaration belongs in; nothing in any other crate. A `const` on `file!()` rather than the
/// macro reading the span's file: it holds on the crate's MSRV, and rust-analyzer does not
/// report it as a false alarm while a file is being moved.
pub(crate) fn check(declared: Declared<'_>) -> TokenStream2 {
    if !binds() {
        return TokenStream2::new();
    }
    let expected = declared.expected();
    let before = format!("[{}] {} is in ", declared.control(), declared.what());
    let after = format!(" — it belongs in src/{expected}, alone");
    quote! {
        const _: () = if !::portaki_sdk::layout::in_place(::core::file!(), #expected) {
            ::core::panic!(::core::concat!(#before, ::core::file!(), #after))
        };
    }
}

/// The crate being compiled asks `portaki-sdk` for `unstable` in its own manifest.
fn binds() -> bool {
    std::env::var_os("CARGO_MANIFEST_DIR")
        .is_some_and(|dir| asks_unstable(&PathBuf::from(dir).join("Cargo.toml")))
}

/// The crate's own manifest asks `portaki-sdk` for `unstable` — inline
/// (`portaki-sdk = { …, features = [… "unstable" …] }`) or as a `[dependencies.portaki-sdk]` table.
pub(crate) fn asks_unstable(manifest: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(manifest) else {
        return false;
    };
    sdk_dependency_asks_unstable(&text)
}

fn sdk_dependency_asks_unstable(text: &str) -> bool {
    let mut in_sdk_table = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_sdk_table = line == "[dependencies.portaki-sdk]";
            continue;
        }
        let names_sdk = line
            .strip_prefix("portaki-sdk")
            .is_some_and(|rest| rest.trim_start().starts_with('='));
        if (in_sdk_table || names_sdk) && line.contains("\"unstable\"") {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::{sdk_dependency_asks_unstable, snake, Declared};

    #[test]
    fn each_declaration_has_one_file() {
        let surface = |context, id| Declared::Surface { context, id }.expected();
        assert_eq!(surface("host", "main"), "host/settings.rs");
        assert_eq!(surface("host", "reports"), "host/reports.rs");
        assert_eq!(surface("guest", "home.card"), "guest/home_card.rs");
        assert_eq!(surface("guest", "guest.form"), "guest/form.rs");
        assert_eq!(
            Declared::Operation("applyTemplate").expected(),
            "commands/apply_template.rs"
        );
        assert_eq!(
            Declared::Connector("open-weather").expected(),
            "connectors/open_weather.rs"
        );
        assert_eq!(Declared::EmailVars.expected(), "email/vars.rs");
        assert_eq!(snake("emailContext"), "email_context");
    }

    #[test]
    fn only_a_crate_asking_unstable_is_bound() {
        assert!(sdk_dependency_asks_unstable(
            "[dependencies]\nportaki-sdk = { version = \"10\", features = [\"repo\", \"unstable\"] }\n"
        ));
        assert!(sdk_dependency_asks_unstable(
            "[dependencies.portaki-sdk]\nversion = \"10\"\nfeatures = [\"unstable\"]\n"
        ));
        assert!(!sdk_dependency_asks_unstable(
            "[dependencies]\nportaki-sdk = { version = \"10\", features = [\"repo\"] }\nother = { features = [\"unstable\"] }\n"
        ));
        assert!(!sdk_dependency_asks_unstable(
            "[dependencies]\nportaki-sdk-macros = { features = [\"unstable\"] }\n"
        ));
    }
}
