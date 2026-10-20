//! The module budget: one layout, and a ceiling on complexity — `portaki check` refuses an SDK 11
//! module (`portaki-sdk` feature `unstable`) that goes past it, and reports it on the others.
//!
//! **Layout** — `src/` holds only these, each a file or, past [`FILE_MAX`] lines, a folder:
//! `lib` (the declarations: `portaki_module!`, capabilities, choices, items, collections,
//! records), `config` (the settings and their checks), `host`, `guest`, `email` (views and
//! mails), `domain` (business logic: no SDUI, no host call), `connectors`, `commands` (the
//! module's own operations). Texts live in the bundles.
//!
//! **Complexity** — clippy with these ceilings, a warning being a failure:
//! [`COGNITIVE`] cognitive complexity, [`FN_LINES`] lines per function, [`ARGS`] arguments,
//! [`NESTING`] nested blocks; and [`FILE_MAX`] code lines per file. An exception is local and
//! written down: `#[allow(clippy::too_many_lines, reason = "…")]` (a bare `allow` is refused,
//! `clippy::allow_attributes_without_reason`), or for a file, a first line
//! `// portaki: allow(file-length, reason = "…")`.

use std::path::{Path, PathBuf};

use anyhow::{bail, Result};

/// Cognitive complexity of one function.
pub const COGNITIVE: u32 = 15;
/// Lines of one function.
pub const FN_LINES: u32 = 80;
/// Arguments of one function.
pub const ARGS: u32 = 5;
/// Nested blocks.
pub const NESTING: u32 = 5;
/// Code lines of one file (tests, blanks and comments aside) before it becomes a folder.
pub const FILE_MAX: usize = 300;

/// What `src/` may hold.
pub const LAYOUT: [&str; 8] = [
    "lib",
    "config",
    "host",
    "guest",
    "email",
    "domain",
    "connectors",
    "commands",
];

const FILE_ALLOW: &str = "// portaki: allow(file-length, reason =";

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
/// up), written under `target/portaki/clippy/` — the directory to give `CLIPPY_CONF_DIR`.
pub fn clippy_conf_dir(module_root: &Path) -> Result<PathBuf> {
    let own = module_root
        .ancestors()
        .map(|dir| dir.join("clippy.toml"))
        .find(|path| path.exists())
        .map(std::fs::read_to_string)
        .transpose()?
        .unwrap_or_default();
    let mut conf = own.clone();
    for (key, value) in [
        ("cognitive-complexity-threshold", COGNITIVE),
        ("too-many-lines-threshold", FN_LINES),
        ("too-many-arguments-threshold", ARGS),
        ("excessive-nesting-threshold", NESTING),
    ] {
        if !own.contains(key) {
            conf.push_str(&format!("\n{key} = {value}\n"));
        }
    }
    let dir = module_root.join("target/portaki/clippy");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("clippy.toml"), conf)?;
    Ok(dir)
}

/// Layout and file length; `Err` with every problem when `enforced`, else they are only shown.
pub fn check_files(module_root: &Path, enforced: bool) -> Result<()> {
    let problems = problems(&module_root.join("src"));
    if problems.is_empty() {
        return Ok(());
    }
    if enforced {
        bail!("module budget:\n  - {}", problems.join("\n  - "));
    }
    crate::ui::warn(format!(
        "module budget (not enforced before SDK 11): {} problem(s)\n  - {}",
        problems.len(),
        problems.join("\n  - ")
    ));
    Ok(())
}

fn problems(src: &Path) -> Vec<String> {
    let mut problems = Vec::new();
    let Ok(entries) = std::fs::read_dir(src) else {
        return problems;
    };
    let mut entries: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    entries.sort();
    for path in &entries {
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        let rust = path.is_dir() || path.extension().is_some_and(|e| e == "rs");
        if rust && !LAYOUT.contains(&stem) {
            problems.push(format!(
                "src/{}: outside the module layout ({}) — business logic goes in domain",
                path.file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default(),
                LAYOUT.join(", ")
            ));
        }
    }
    for file in rust_files(src) {
        let text = std::fs::read_to_string(&file).unwrap_or_default();
        let lines = code_lines(&text);
        if lines > FILE_MAX
            && !text
                .lines()
                .next()
                .unwrap_or_default()
                .starts_with(FILE_ALLOW)
        {
            problems.push(format!(
                "{}: {lines} code lines, more than {FILE_MAX} — make it a folder, or write \
                 `{FILE_ALLOW} \"…\")` on its first line",
                file.strip_prefix(src.parent().unwrap_or(src))
                    .unwrap_or(&file)
                    .display()
            ));
        }
    }
    problems
}

fn rust_files(dir: &Path) -> Vec<PathBuf> {
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

    #[test]
    fn a_long_file_and_a_stray_file_are_problems() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("lib.rs"), "fn a() {}\n").unwrap();
        std::fs::write(src.join("helpers.rs"), "fn b() {}\n").unwrap();
        std::fs::write(src.join("host.rs"), "let x = 1;\n".repeat(FILE_MAX + 1)).unwrap();
        let found = problems(&src);
        assert_eq!(found.len(), 2, "{found:?}");
        assert!(found[0].contains("src/helpers.rs: outside the module layout"));
        assert!(found[1].contains("src/host.rs: 301 code lines"));
    }

    #[test]
    fn a_written_exception_and_tests_do_not_count() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir_all(src.join("guest")).unwrap();
        let long = "let x = 1;\n".repeat(FILE_MAX + 1);
        std::fs::write(
            src.join("domain.rs"),
            format!("{FILE_ALLOW} \"one table\")\n{long}"),
        )
        .unwrap();
        std::fs::write(
            src.join("guest/body.rs"),
            format!("fn a() {{}}\n#[cfg(test)]\n{long}"),
        )
        .unwrap();
        assert!(problems(&src).is_empty());
    }

    #[test]
    fn the_conf_keeps_the_modules_own_rules() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("clippy.toml"),
            "disallowed-methods = []\ntoo-many-lines-threshold = 60\n",
        )
        .unwrap();
        let conf =
            std::fs::read_to_string(clippy_conf_dir(dir.path()).unwrap().join("clippy.toml"))
                .unwrap();
        assert!(conf.contains("disallowed-methods"));
        assert!(conf.contains("cognitive-complexity-threshold = 15"));
        assert!(
            !conf.contains("too-many-lines-threshold = 80"),
            "the module's stricter value wins"
        );
    }
}
