//! The module budget: one file per surface or responsibility, and a ceiling on complexity —
//! `portaki check` refuses an SDK 11 module (`portaki-sdk` feature `unstable`) that breaks it,
//! and reports it on the others. Each problem is named by its control, in brackets.
//!
//! **Layout** (`[module-layout]`) — `src/` holds `lib.rs` (the declarations only), `config.rs`
//! (or `config/`), and folders: `host/` and `guest/` (one surface per file), `domain/` (one
//! business concept per file), `commands/`, `connectors/`, `email/` (one element per file); a
//! folder may have a `mod.rs` that only gathers.
//!
//! - `[one-surface-per-file]` — a file declaring two surfaces is refused, naming both;
//! - `[surface-file-name]` — the file is named after its surface (`home.card` → `home_card.rs`
//!   or `card.rs`; the host `main` may be `settings.rs`), in `host/` or `guest/` by its side;
//! - `[one-command-per-file]` — a `#[command]` / `#[query]` lives in `commands/`, one per file;
//! - `[one-connector-per-file]`, `[one-email-per-file]` — likewise in `connectors/`, `email/`;
//! - `[domain-purity]` — `domain/` never names SDUI (`sdui::`, `Surface`, `Component`) nor the
//!   host (`host::`, `t!`, `bundle_text!`);
//! - `[file-length]` — at most [`FILE_MAX`] code lines per file, the safety net.
//!
//! **Complexity** — clippy on the module's code with these ceilings, a warning being a failure:
//! [`COGNITIVE`] cognitive complexity, [`FN_LINES`] lines per function, [`ARGS`] arguments,
//! [`NESTING`] nested blocks. An exception is local and written down:
//! `#[allow(clippy::too_many_lines, reason = "…")]` (a bare `allow` is refused,
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

/// The layout controls; `Err` with every problem when `enforced`, else they are only shown.
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
        surfaces_of(&shown, &folder, &stem, code, &mut problems);
        let operations = attribute_bodies(code, &["command", "query"]);
        if !operations.is_empty() && folder != "commands" {
            problems.push(format!(
                "[one-command-per-file] {shown}: {} — operations live in src/commands/, one per file",
                operations.iter().map(|b| name_of(b)).collect::<Vec<_>>().join(", ")
            ));
        } else if operations.len() > 1 {
            problems.push(format!(
                "[one-command-per-file] {shown}: {} — one file each in src/commands/",
                operations
                    .iter()
                    .map(|b| name_of(b))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if folder == "connectors"
            && attribute_bodies(code, &["custom_connector", "connector"]).len() > 1
        {
            problems.push(format!(
                "[one-connector-per-file] {shown}: one connector per file"
            ));
        }
        if folder == "email" && stem != "mod" && code.matches("pub fn ").count() > 1 {
            problems.push(format!("[one-email-per-file] {shown}: one email per file"));
        }
        if folder == "domain" {
            for (n, line) in code.lines().enumerate() {
                let line = line.trim();
                if line.starts_with("//") {
                    continue;
                }
                if let Some(word) = DOMAIN_FORBIDDEN.iter().find(|w| names(line, w)) {
                    problems.push(format!(
                        "[domain-purity] {shown}:{}: `{word}` — domain/ holds no SDUI and calls no host; move it to host/, guest/ or email/",
                        n + 1
                    ));
                }
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

/// What `domain/` may not name.
const DOMAIN_FORBIDDEN: [&str; 6] = [
    "sdui::",
    "Surface",
    "Component",
    "host::",
    "t!(",
    "bundle_text!",
];

/// `word` stands on its own in `line`: `format!(` does not name `t!(`.
fn names(line: &str, word: &str) -> bool {
    line.match_indices(word).any(|(at, _)| {
        !line[..at]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_alphanumeric() || c == '_')
    })
}

/// The surfaces a file declares: one at most, named after it, in its side's folder.
fn surfaces_of(shown: &str, folder: &str, stem: &str, code: &str, problems: &mut Vec<String>) {
    let surfaces: Vec<(String, String)> = attribute_bodies(code, &["surface"])
        .iter()
        .filter_map(|body| {
            let side = body
                .trim_start()
                .split([',', ' ', ')'])
                .next()?
                .trim_matches('"')
                .to_string();
            Some((side, quoted_after(body, "id")?))
        })
        .collect();
    if surfaces.len() > 1 {
        problems.push(format!(
            "[one-surface-per-file] {shown}: {} — one file each: {}",
            surfaces
                .iter()
                .map(|(side, id)| format!("{side} {id}"))
                .collect::<Vec<_>>()
                .join(" and "),
            surfaces
                .iter()
                .map(|(side, id)| format!("src/{side}/{}.rs", file_name_of(id)))
                .collect::<Vec<_>>()
                .join(", ")
        ));
        return;
    }
    let Some((side, id)) = surfaces.first() else {
        return;
    };
    let named = stem == file_name_of(id)
        || stem == id.rsplit('.').next().unwrap_or(id).replace('-', "_")
        || (id == "main" && stem == "settings");
    if folder != side.as_str() || !named {
        problems.push(format!(
            "[surface-file-name] {shown}: {side} surface `{id}` — expected src/{side}/{}.rs",
            if id == "main" {
                "settings".to_string()
            } else {
                file_name_of(id)
            }
        ));
    }
}

/// `home.card` → `home_card`.
fn file_name_of(id: &str) -> String {
    id.replace(['.', '-'], "_")
}

/// The bodies of `#[name(…)]` / `#[portaki_sdk::name(…)]` attributes, in order.
fn attribute_bodies(code: &str, names: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    for name in names {
        for prefix in [format!("#[portaki_sdk::{name}("), format!("#[{name}(")] {
            let mut rest = code;
            while let Some(at) = rest.find(&prefix) {
                let after = &rest[at + prefix.len()..];
                let mut depth = 1;
                let end = after
                    .char_indices()
                    .find(|(_, c)| {
                        match c {
                            '(' => depth += 1,
                            ')' => depth -= 1,
                            _ => {}
                        }
                        depth == 0
                    })
                    .map_or(after.len(), |(i, _)| i);
                out.push(after[..end].to_string());
                rest = &after[end..];
            }
        }
    }
    out
}

fn quoted_after(body: &str, key: &str) -> Option<String> {
    let at = body.find(&format!("{key} ="))?;
    let rest = &body[at..];
    let start = rest.find('"')? + 1;
    let end = rest[start..].find('"')? + start;
    Some(rest[start..end].to_string())
}

fn name_of(body: &str) -> String {
    quoted_after(body, "name").unwrap_or_else(|| "?".into())
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
    fn the_layout_is_folders_with_one_thing_per_file() {
        let dir = src_with(&[
            ("lib.rs", "mod host;\n"),
            ("helpers.rs", "fn b() {}\n"),
            ("host.rs", "fn c() {}\n"),
            ("guest/mod.rs", "mod home_card;\n"),
            ("guest/home_card.rs", "#[portaki_sdk::surface(guest, id = \"home.card\")]\nfn a() {}\n#[portaki_sdk::surface(guest, id = \"explore.detail\", path = \"x\")]\nfn b() {}\n"),
            ("host/stats.rs", "#[portaki_sdk::surface(host, id = \"main\")]\nfn a() {}\n#[portaki_sdk::command(name = \"save\")]\nfn s() {}\n"),
            ("domain/rule.rs", "use portaki_sdk::sdui::primitives::Text;\n"),
        ]);
        let found = problems(&dir.path().join("src"));
        let has = |needle: &str| found.iter().any(|p| p.contains(needle));
        assert!(
            has("[module-layout] src/helpers.rs: business logic goes in src/domain/"),
            "{found:#?}"
        );
        assert!(
            has("[module-layout] src/host.rs: host is a folder"),
            "{found:#?}"
        );
        assert!(has("[one-surface-per-file] src/guest/home_card.rs: guest home.card and guest explore.detail — one file each: src/guest/home_card.rs, src/guest/explore_detail.rs"), "{found:#?}");
        assert!(has("[surface-file-name] src/host/stats.rs: host surface `main` — expected src/host/settings.rs"), "{found:#?}");
        assert!(
            has(
                "[one-command-per-file] src/host/stats.rs: save — operations live in src/commands/"
            ),
            "{found:#?}"
        );
        assert!(
            has("[domain-purity] src/domain/rule.rs:1: `sdui::`"),
            "{found:#?}"
        );
        assert_eq!(found.len(), 6, "{found:#?}");
    }

    #[test]
    fn a_tidy_module_passes_and_a_written_exception_holds() {
        let long = "let x = 1;\n".repeat(FILE_MAX + 1);
        let dir = src_with(&[
            ("lib.rs", "mod guest;\n"),
            ("config.rs", ""),
            ("guest/form.rs", "#[portaki_sdk::surface(guest, id = \"guest.form\", path = \"x\")]\nfn a() {}\n"),
            ("host/settings.rs", "#[portaki_sdk::surface(host, id = \"main\", placement = HostPlacement::X)]\nfn a() {}\n"),
            ("commands/apply.rs", "#[portaki_sdk::command(name = \"apply\", example(label = \"x\", input = \"{}\"))]\nfn a() {}\n"),
            ("domain/label.rs", "fn a() -> String { format!(\"{}\", 1) }\n"),
            ("domain/table.rs", &format!("{FILE_ALLOW} \"one table\")\n{long}")),
        ]);
        assert!(problems(&dir.path().join("src")).is_empty());
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
