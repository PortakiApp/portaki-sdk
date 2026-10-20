//! The budget's exceptions: every `#[allow]` / `#[expect]` of a budget lint and every
//! `// portaki: allow(file-length, …)` of a module, counted against a ceiling the module does not
//! set, and listed in `EXCEPTIONS.md` so that adding one changes a file a code owner reviews.
//!
//! - The ceiling is [`DEFAULT_CAP`], or `exceptions = N` in `portaki-budget.toml` at the root of
//!   the repository holding the module — never in the module's own directory.
//! - A file- or crate-wide `#![allow(…)]` of a budget lint is refused outright.
//! - `portaki check --report` prints the list; `EXCEPTIONS.md` must be that list, word for word,
//!   whenever the module has an exception.

use std::path::Path;

use syn::punctuated::Punctuated;
use syn::visit::Visit;

/// Exceptions a module may hold when the repository says nothing.
pub const DEFAULT_CAP: usize = 3;
/// Where the repository sets the ceiling.
pub const CAP_FILE: &str = "portaki-budget.toml";
/// The list a module with exceptions commits.
pub const LOCK_FILE: &str = "EXCEPTIONS.md";

/// What an exception may silence: the budget's lints, and the groups that hold them.
const BUDGET_LINTS: [&str; 11] = [
    "clippy::cognitive_complexity",
    "clippy::too_many_lines",
    "clippy::too_many_arguments",
    "clippy::excessive_nesting",
    "clippy::allow_attributes_without_reason",
    "clippy::all",
    "clippy::complexity",
    "clippy::pedantic",
    "clippy::nursery",
    "clippy::restriction",
    "warnings",
];

const FILE_ALLOW: &str = "// portaki: allow(file-length, reason =";

/// One exception, where it is and why.
#[derive(Debug, PartialEq)]
pub struct Exception {
    pub file: String,
    pub item: String,
    pub lints: String,
    pub reason: String,
}

/// The module's exceptions, and the allows refused whatever the count.
pub fn collect(src: &Path) -> (Vec<Exception>, Vec<String>) {
    let mut found = Vec::new();
    let mut refused = Vec::new();
    for file in super::budget::rust_files(src) {
        let shown = format!("src/{}", file.strip_prefix(src).unwrap_or(&file).display());
        let text = std::fs::read_to_string(&file).unwrap_or_default();
        if let Some(rest) = text.lines().next().and_then(|l| l.strip_prefix(FILE_ALLOW)) {
            found.push(Exception {
                file: shown.clone(),
                item: "(file)".into(),
                lints: "file-length".into(),
                reason: rest
                    .trim()
                    .trim_end_matches(')')
                    .trim()
                    .trim_matches('"')
                    .into(),
            });
        }
        let Ok(parsed) = syn::parse_file(&text) else {
            continue;
        };
        let mut allows = Allows {
            file: &shown,
            items: Vec::new(),
            found: &mut found,
            refused: &mut refused,
        };
        allows.visit_file(&parsed);
    }
    (found, refused)
}

/// The ceiling for the module at `module_root`: the repository's, not the module's.
pub fn cap(module_root: &Path) -> usize {
    let module_root = module_root
        .canonicalize()
        .unwrap_or_else(|_| module_root.to_path_buf());
    let Some(repo) = module_root
        .ancestors()
        .find(|dir| dir.join(".git").exists())
    else {
        return DEFAULT_CAP;
    };
    if repo == module_root {
        return DEFAULT_CAP;
    }
    std::fs::read_to_string(repo.join(CAP_FILE))
        .ok()
        .and_then(|text| text.parse::<toml_edit::DocumentMut>().ok())
        .and_then(|doc| doc.get("exceptions")?.as_integer())
        .and_then(|n| usize::try_from(n).ok())
        .unwrap_or(DEFAULT_CAP)
}

/// The markdown list `portaki check --report` prints and `EXCEPTIONS.md` holds.
pub fn report(module_root: &Path) -> String {
    let (found, _) = collect(&module_root.join("src"));
    let name = module_root
        .canonicalize()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_default();
    let mut out = format!(
        "## Budget exceptions — {name}\n\n{} of the {} the repository allows.\n",
        found.len(),
        cap(module_root)
    );
    if found.is_empty() {
        return out;
    }
    out.push_str("\n| File | Item | Silences | Reason |\n|---|---|---|---|\n");
    for e in &found {
        out.push_str(&format!(
            "| `{}` | `{}` | `{}` | {} |\n",
            e.file,
            e.item,
            e.lints,
            e.reason.replace('|', "\\|")
        ));
    }
    out
}

/// `[exceptions]` problems: a refused allow, past the ceiling, or a stale `EXCEPTIONS.md`.
pub fn problems(module_root: &Path) -> Vec<String> {
    let (found, mut problems) = collect(&module_root.join("src"));
    let cap = cap(module_root);
    if found.len() > cap {
        problems.push(format!(
            "[exceptions] {} exceptions, the repository allows {cap} — reasons do not raise the \
             ceiling: split the code instead (portaki check --report lists them)",
            found.len()
        ));
    }
    let lock = std::fs::read_to_string(module_root.join(LOCK_FILE)).ok();
    let expected = report(module_root);
    let stale = match &lock {
        Some(text) => text.trim() != expected.trim(),
        None => !found.is_empty(),
    };
    if stale {
        problems.push(format!(
            "[exceptions] {LOCK_FILE} does not list the module's exceptions — write it: \
             portaki check --report > {LOCK_FILE}"
        ));
    }
    problems
}

struct Allows<'a> {
    file: &'a str,
    items: Vec<String>,
    found: &'a mut Vec<Exception>,
    refused: &'a mut Vec<String>,
}

impl<'ast> Visit<'ast> for Allows<'_> {
    fn visit_item_mod(&mut self, module: &'ast syn::ItemMod) {
        if !is_test(&module.attrs) {
            syn::visit::visit_item_mod(self, module);
        }
    }

    fn visit_item_fn(&mut self, function: &'ast syn::ItemFn) {
        self.items.push(function.sig.ident.to_string());
        syn::visit::visit_item_fn(self, function);
        self.items.pop();
    }

    fn visit_impl_item_fn(&mut self, function: &'ast syn::ImplItemFn) {
        self.items.push(function.sig.ident.to_string());
        syn::visit::visit_impl_item_fn(self, function);
        self.items.pop();
    }

    fn visit_attribute(&mut self, attr: &'ast syn::Attribute) {
        if !(attr.path().is_ident("allow") || attr.path().is_ident("expect")) {
            return;
        }
        let Ok(metas) =
            attr.parse_args_with(Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated)
        else {
            return;
        };
        let mut lints = Vec::new();
        let mut reason = String::new();
        for meta in &metas {
            match meta {
                syn::Meta::Path(path) => {
                    let name = path_string(path);
                    if BUDGET_LINTS.contains(&name.as_str()) {
                        lints.push(name);
                    }
                }
                syn::Meta::NameValue(pair) if pair.path.is_ident("reason") => {
                    if let syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(text),
                        ..
                    }) = &pair.value
                    {
                        reason = text.value();
                    }
                }
                _ => {}
            }
        }
        if lints.is_empty() {
            return;
        }
        let lints = lints.join(", ");
        if matches!(attr.style, syn::AttrStyle::Inner(_)) {
            self.refused.push(format!(
                "[exceptions] {}: `#![allow({lints})]` covers a whole file — allow one item, with \
                 a reason",
                self.file
            ));
            return;
        }
        self.found.push(Exception {
            file: self.file.to_string(),
            item: self
                .items
                .last()
                .cloned()
                .unwrap_or_else(|| "(item)".into()),
            lints,
            reason,
        });
    }
}

fn path_string(path: &syn::Path) -> String {
    path.segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}

/// `#[cfg(test)]`: not the module's code.
pub fn is_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("cfg")
            && attr
                .parse_args::<syn::Ident>()
                .is_ok_and(|ident| ident == "test")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn module(files: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for (path, text) in files {
            let path = dir.path().join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        dir
    }

    const LONG_FN: &str =
        "#[allow(clippy::too_many_lines, reason = \"one table\")]\nfn table() {}\n";

    #[test]
    fn exceptions_are_counted_listed_and_capped() {
        let dir = module(&[
            ("src/domain/a.rs", LONG_FN),
            (
                "src/domain/b.rs",
                "#[allow(dead_code)]\nfn unused() {}\nimpl X {\n#[expect(clippy::complexity, reason = \"x\")]\nfn m() {}\n}\n",
            ),
            ("src/domain/c.rs", "// portaki: allow(file-length, reason = \"a table\")\n"),
            ("src/domain/d.rs", LONG_FN),
            ("src/host/mod.rs", "#![allow(clippy::too_many_lines)]\n"),
            (
                "src/guest/x.rs",
                "#[cfg(test)]\nmod tests {\n#[allow(clippy::too_many_lines)]\nfn t() {}\n}\n",
            ),
        ]);
        let (found, refused) = collect(&dir.path().join("src"));
        assert_eq!(found.len(), 4, "{found:#?}");
        assert!(found.contains(&Exception {
            file: "src/domain/b.rs".into(),
            item: "m".into(),
            lints: "clippy::complexity".into(),
            reason: "x".into(),
        }));
        assert_eq!(found[2].reason, "a table");
        assert_eq!(refused.len(), 1, "{refused:#?}");
        assert!(refused[0].starts_with("[exceptions] src/host/mod.rs: `#![allow("));

        let found = problems(dir.path());
        assert!(
            found
                .iter()
                .any(|p| p.starts_with("[exceptions] 4 exceptions, the repository allows 3")),
            "{found:#?}"
        );
        assert!(
            found
                .iter()
                .any(|p| p.contains("EXCEPTIONS.md does not list")),
            "{found:#?}"
        );
    }

    #[test]
    fn the_lock_is_the_report_and_the_ceiling_is_the_repositorys() {
        let repo = module(&[
            (".git/HEAD", ""),
            (CAP_FILE, "exceptions = 1\n"),
            ("modules/m/src/domain/a.rs", LONG_FN),
            ("modules/m/src/domain/b.rs", LONG_FN),
            ("modules/m/portaki-budget.toml", "exceptions = 99\n"),
        ]);
        let root = repo.path().join("modules/m");
        assert_eq!(cap(&root), 1, "the module's own file is ignored");
        std::fs::write(root.join(LOCK_FILE), report(&root)).unwrap();
        let found = problems(&root);
        assert_eq!(found.len(), 1, "{found:#?}");
        assert!(found[0].contains("the repository allows 1"));
        assert!(report(&root)
            .contains("| `src/domain/a.rs` | `table` | `clippy::too_many_lines` | one table |"));
    }
}
