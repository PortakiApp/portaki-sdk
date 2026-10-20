//! The module budget: one file per surface or responsibility, and a ceiling on complexity.
//! `portaki build` (so `check` and `release`) enforces it on an SDK 11 module (`portaki-sdk`
//! feature `unstable`), with no flag to turn it off, and shows it to the others. Each problem is
//! named by its control, in brackets.
//!
//! Three levels hold it:
//!
//! 1. **Compilation** — `#[surface]`, `#[command]` / `#[query]`, `#[connector]` /
//!    `#[custom_connector]`, `#[email_vars]` / `#[email_blocks]` refuse to compile outside their
//!    file (`[one-surface-per-file]`, `[one-command-per-file]`, `[one-connector-per-file]`,
//!    `[one-email-per-file]`; table in `portaki-sdk-macros/src/layout.rs`).
//! 2. **Packaging** — here, before `cargo build`: what Rust cannot say.
//!    - `[module-layout]` — `src/` holds `lib.rs` (the declarations only), `config.rs` (or
//!      `config/`), and folders: `host/`, `guest/`, `domain/`, `commands/`, `connectors/`,
//!      `email/`; a folder's `mod.rs` only gathers;
//!    - `[one-email-per-file]` — a file of `email/` writes one email (one `pub fn`);
//!    - `[domain-purity]` — `domain/` names no `portaki_sdk::{sdui, host, prelude}`, no
//!      `crate::{host, guest, email, commands}`, no `Surface` / `Component`, no `t!` /
//!      `bundle_text!`. Read with syn: Rust cannot scope an import ban to one module, and a macro
//!      reading `domain/` would go stale under incremental compilation;
//!    - `[sdk-only]` — `__sdk` marks the SDK's own derived operations, never a module's;
//!    - `[file-length]` — at most [`FILE_MAX`] code lines per file, the safety net;
//!    - complexity — clippy on the module's code with these ceilings, a warning being a failure:
//!      [`COGNITIVE`] cognitive complexity, [`FN_LINES`] lines per function, [`ARGS`] arguments,
//!      [`NESTING`] nested blocks. A module's `clippy.toml` may lower them, never raise them.
//! 3. **Exceptions** (`[exceptions]`, see [`super::exceptions`]) — `#[allow(…, reason = "…")]`
//!    or a first line `// portaki: allow(file-length, reason = "…")`, counted against the
//!    repository's ceiling and listed in `EXCEPTIONS.md`.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use syn::visit::Visit;

use crate::ui;

/// Cognitive complexity of one function.
pub const COGNITIVE: u32 = 15;
/// Lines of one function.
pub const FN_LINES: u32 = 80;
/// Arguments of one function.
pub const ARGS: u32 = 5;
/// Nested blocks.
pub const NESTING: u32 = 5;
/// Code lines of one file (tests, blanks and comments aside).
pub const FILE_MAX: usize = 300;

/// The files `src/` may hold at its root.
pub const ROOT_FILES: [&str; 2] = ["lib.rs", "config.rs"];
/// The folders `src/` may hold.
pub const FOLDERS: [&str; 7] = [
    "config",
    "host",
    "guest",
    "domain",
    "commands",
    "connectors",
    "email",
];

const FILE_ALLOW: &str = "// portaki: allow(file-length, reason =";

/// The module asks `portaki-sdk` for `unstable`: the budget binds it.
pub fn binds(module_root: &Path) -> Result<bool> {
    Ok(crate::oci::pack::sdk_features(module_root)?
        .iter()
        .any(|feature| feature == "unstable"))
}

/// The whole budget, as `portaki build` runs it: refused on SDK 11, shown otherwise.
pub fn enforce(module_root: &Path) -> Result<()> {
    let binding = binds(module_root)?;
    check_files(module_root, binding)?;
    if binding {
        complexity_step(module_root)?;
    }
    Ok(())
}

/// The clippy flags of the budget.
pub fn clippy_flags() -> Vec<String> {
    [
        "clippy::cognitive_complexity",
        "clippy::too_many_lines",
        "clippy::too_many_arguments",
        "clippy::excessive_nesting",
        "clippy::allow_attributes_without_reason",
    ]
    .iter()
    .flat_map(|lint| ["-W".to_string(), (*lint).to_string()])
    .collect()
}

/// A `clippy.toml` with the budget's ceilings, after the module's own (found from `module_root`
/// up), written under `target/portaki/clippy/` — the directory to give `CLIPPY_CONF_DIR`. A
/// stricter value of the module's stays; a laxer one is replaced.
pub fn clippy_conf_dir(module_root: &Path) -> Result<PathBuf> {
    let own = module_root
        .ancestors()
        .map(|dir| dir.join("clippy.toml"))
        .find(|path| path.exists())
        .map(std::fs::read_to_string)
        .transpose()?
        .unwrap_or_default();
    let ceilings = [
        ("cognitive-complexity-threshold", COGNITIVE),
        ("too-many-lines-threshold", FN_LINES),
        ("too-many-arguments-threshold", ARGS),
        ("excessive-nesting-threshold", NESTING),
    ];
    let value_of = |line: &str, key: &str| -> Option<u32> {
        let (name, value) = line.split_once('=')?;
        (name.trim() == key).then(|| value.trim().parse().ok())?
    };
    let mut conf: Vec<String> = Vec::new();
    for line in own.lines() {
        let laxer = ceilings.iter().any(|(key, ceiling)| {
            value_of(line, key).is_some_and(|value| value > *ceiling)
                || (line.split('=').next().map(str::trim) == Some(*key)
                    && value_of(line, key).is_none())
        });
        if !laxer {
            conf.push(line.to_string());
        }
    }
    for (key, ceiling) in ceilings {
        if !conf.iter().any(|line| value_of(line, key).is_some()) {
            conf.push(format!("{key} = {ceiling}"));
        }
    }
    let dir = module_root.join("target/portaki/clippy");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("clippy.toml"), conf.join("\n") + "\n")?;
    Ok(dir)
}

/// The complexity budget, on the module's own code (`--lib`): tests and shared harnesses are not
/// held to it.
fn complexity_step(module_root: &Path) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.current_dir(module_root)
        .env("CLIPPY_CONF_DIR", clippy_conf_dir(module_root)?)
        .arg("clippy")
        .arg("--lib")
        .arg("--")
        .arg("-D")
        .arg("warnings")
        .args(clippy_flags());
    ui::command("module budget (clippy --lib)", &mut cmd).with_context(|| {
        crate::tr!(
            "module budget — a declaration in the wrong file, or a ceiling past: move it where the \
             error says, split the function, or allow it there with a reason",
            "budget du module — une déclaration hors de son fichier, ou un plafond dépassé : \
             déplacez-la où l'erreur le dit, découpez la fonction, ou autorisez-la sur place avec \
             une raison"
        )
    })
}

/// The packaging controls and the exceptions; `Err` with every problem when `enforced`, else
/// they are only shown.
pub fn check_files(module_root: &Path, enforced: bool) -> Result<()> {
    let mut problems = problems(&module_root.join("src"));
    problems.extend(super::exceptions::problems(module_root));
    if problems.is_empty() {
        return Ok(());
    }
    if enforced {
        bail!("module budget:\n  - {}", problems.join("\n  - "));
    }
    ui::warn(format!(
        "module budget (not enforced before SDK 11): {} problem(s)\n  - {}",
        problems.len(),
        problems.join("\n  - ")
    ));
    Ok(())
}

fn problems(src: &Path) -> Vec<String> {
    let mut problems = Vec::new();
    let mut entries: Vec<PathBuf> = std::fs::read_dir(src)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .collect();
    entries.sort();
    for path in &entries {
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        let ok = if path.is_dir() {
            FOLDERS.contains(&name)
        } else {
            !name.ends_with(".rs") || ROOT_FILES.contains(&name)
        };
        if !ok {
            let stem = name.trim_end_matches(".rs");
            let hint = if FOLDERS.contains(&stem) {
                format!("{stem} is a folder: src/{stem}/, one element per file")
            } else {
                "business logic goes in src/domain/, one concept per file".to_string()
            };
            problems.push(format!("[module-layout] src/{name}: {hint}"));
        }
    }
    for file in rust_files(src) {
        let rel = file.strip_prefix(src).unwrap_or(&file).to_path_buf();
        let shown = format!("src/{}", rel.display());
        let folder = rel
            .components()
            .next()
            .and_then(|c| c.as_os_str().to_str())
            .unwrap_or_default()
            .to_string();
        let stem = file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();
        let text = std::fs::read_to_string(&file).unwrap_or_default();
        let code = text.split("#[cfg(test)]").next().unwrap_or_default();
        if code.contains("__sdk") {
            problems.push(format!(
                "[sdk-only] {shown}: `__sdk` marks the operations the SDK derives — a module \
                 declares its own in src/commands/"
            ));
        }
        if folder == "email" && stem != "mod" && code.matches("pub fn ").count() > 1 {
            problems.push(format!("[one-email-per-file] {shown}: one email per file"));
        }
        if folder == "domain" {
            for named in impurities(&text) {
                problems.push(format!(
                    "[domain-purity] {shown}: `{named}` — domain/ holds no SDUI and calls no \
                     host; move it to host/, guest/ or email/"
                ));
            }
        }
        let lines = code_lines(&text);
        if lines > FILE_MAX
            && !text
                .lines()
                .next()
                .unwrap_or_default()
                .starts_with(FILE_ALLOW)
        {
            problems.push(format!(
                "[file-length] {shown}: {lines} code lines, more than {FILE_MAX} — split it by \
                 responsibility, or write `{FILE_ALLOW} \"…\")` on its first line"
            ));
        }
    }
    problems
}

/// What a `domain/` file names that is SDUI, the host, or another side of the module.
fn impurities(text: &str) -> Vec<String> {
    let Ok(file) = syn::parse_file(text) else {
        return Vec::new();
    };
    let mut purity = Purity::default();
    purity.visit_file(&file);
    purity.found.dedup();
    purity.found
}

#[derive(Default)]
struct Purity {
    found: Vec<String>,
}

impl Purity {
    fn check(&mut self, segments: &[String]) {
        let first = segments.first().map(String::as_str);
        let second = segments.get(1).map(String::as_str);
        let impure = matches!(
            (first, second),
            (Some("portaki_sdk"), Some("sdui" | "host" | "prelude"))
                | (Some("crate"), Some("host" | "guest" | "email" | "commands"))
        ) || segments.iter().any(|s| s == "Surface" || s == "Component");
        if impure {
            self.found.push(segments.join("::"));
        }
    }
}

impl<'ast> Visit<'ast> for Purity {
    fn visit_item_mod(&mut self, module: &'ast syn::ItemMod) {
        if !super::exceptions::is_test(&module.attrs) {
            syn::visit::visit_item_mod(self, module);
        }
    }

    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        let mut paths = Vec::new();
        use_paths(&item.tree, &mut Vec::new(), &mut paths);
        for path in paths {
            self.check(&path);
        }
    }

    fn visit_path(&mut self, path: &'ast syn::Path) {
        let segments: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
        self.check(&segments);
        syn::visit::visit_path(self, path);
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if let Some(last) = mac.path.segments.last() {
            if last.ident == "t" || last.ident == "bundle_text" {
                self.found.push(format!("{}!", last.ident));
            }
        }
        syn::visit::visit_macro(self, mac);
        // `format!(…, t!(…))`: a macro's arguments are tokens to syn until read as expressions.
        if let Ok(args) = mac.parse_body_with(
            syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated,
        ) {
            for arg in &args {
                self.visit_expr(arg);
            }
        }
    }
}

/// Every path a `use` tree names, a glob as `*`.
fn use_paths(tree: &syn::UseTree, prefix: &mut Vec<String>, out: &mut Vec<Vec<String>>) {
    match tree {
        syn::UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            use_paths(&path.tree, prefix, out);
            prefix.pop();
        }
        syn::UseTree::Name(name) => {
            out.push([prefix.clone(), vec![name.ident.to_string()]].concat())
        }
        syn::UseTree::Rename(rename) => {
            out.push([prefix.clone(), vec![rename.ident.to_string()]].concat())
        }
        syn::UseTree::Glob(_) => out.push([prefix.clone(), vec!["*".to_string()]].concat()),
        syn::UseTree::Group(group) => {
            for tree in &group.items {
                use_paths(tree, prefix, out);
            }
        }
    }
}

/// The `.rs` files under `dir`, sorted.
pub fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(rust_files(&path));
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
    out.sort();
    out
}

/// Lines that are code: before the first `#[cfg(test)]`, not blank, not a comment.
pub fn code_lines(text: &str) -> usize {
    let code = text.split("#[cfg(test)]").next().unwrap_or_default();
    code.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("//"))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn src_with(files: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for (path, text) in files {
            let path = dir.path().join("src").join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        dir
    }

    #[test]
    fn the_layout_is_folders_and_the_domain_stays_pure() {
        let dir = src_with(&[
            ("lib.rs", "mod host;\n"),
            ("helpers.rs", "fn b() {}\n"),
            ("host.rs", "fn c() {}\n"),
            ("email/to_host.rs", "pub fn a() {}\npub fn b() {}\n"),
            ("commands/save.rs", "#[portaki_sdk::command(name = \"save\", __sdk)]\nfn s() {}\n"),
            (
                "domain/rule.rs",
                "use portaki_sdk::prelude::*;\nuse portaki_sdk::{sdui::primitives::Text, Result};\n\
                 use crate::guest::row::label;\n\
                 fn a() -> String { format!(\"{}\", t!(\"k\")) }\n\
                 fn b() { ::portaki_sdk::host::time::now(); }\n\
                 // portaki_sdk::sdui in a comment\n\
                 #[cfg(test)]\nmod tests { use portaki_sdk::sdui::x; }\n",
            ),
        ]);
        let found = problems(&dir.path().join("src"));
        let expected = [
            "[module-layout] src/helpers.rs: business logic goes in src/domain/",
            "[module-layout] src/host.rs: host is a folder",
            "[sdk-only] src/commands/save.rs: `__sdk`",
            "[domain-purity] src/domain/rule.rs: `portaki_sdk::prelude::*`",
            "[domain-purity] src/domain/rule.rs: `portaki_sdk::sdui::primitives::Text`",
            "[domain-purity] src/domain/rule.rs: `crate::guest::row::label`",
            "[domain-purity] src/domain/rule.rs: `t!`",
            "[domain-purity] src/domain/rule.rs: `portaki_sdk::host::time::now`",
            "[one-email-per-file] src/email/to_host.rs",
        ];
        for needle in expected {
            assert!(
                found.iter().any(|p| p.starts_with(needle)),
                "{needle}\n{found:#?}"
            );
        }
        assert_eq!(found.len(), expected.len(), "{found:#?}");
    }

    #[test]
    fn a_tidy_module_passes_and_a_written_exception_holds() {
        let long = "let x = 1;\n".repeat(FILE_MAX + 1);
        let dir = src_with(&[
            ("lib.rs", "mod guest;\n"),
            ("config.rs", ""),
            ("guest/form.rs", "fn a() {}\n"),
            ("email/mod.rs", "pub fn cta() {}\npub fn send() {}\n"),
            (
                "domain/label.rs",
                "use crate::Rule;\nfn a() -> String { format!(\"{}\", 1) }\n",
            ),
            (
                "domain/table.rs",
                &format!("{FILE_ALLOW} \"one table\")\n{long}"),
            ),
        ]);
        assert!(problems(&dir.path().join("src")).is_empty());
    }

    #[test]
    fn the_conf_keeps_a_stricter_rule_and_drops_a_laxer_one() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("clippy.toml"),
            "disallowed-methods = []\ntoo-many-lines-threshold = 60\ncognitive-complexity-threshold = 99\n",
        )
        .unwrap();
        let conf =
            std::fs::read_to_string(clippy_conf_dir(dir.path()).unwrap().join("clippy.toml"))
                .unwrap();
        assert!(conf.contains("disallowed-methods"));
        assert!(conf.contains("too-many-lines-threshold = 60"));
        assert!(!conf.contains("too-many-lines-threshold = 80"));
        assert!(conf.contains("cognitive-complexity-threshold = 15"));
        assert!(!conf.contains("= 99"));
    }
}
