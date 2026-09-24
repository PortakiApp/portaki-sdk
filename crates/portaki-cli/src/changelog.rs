//! `changelog` of the published manifest: what is new in this version, for hosts on an older one.
//!
//! `portaki publish --notes` wins; otherwise the version's section of the module's `CHANGELOG.md`
//! (Keep a Changelog or release-please: `## [x.y.z]` or `## x.y.z`, then bullets). Neither says
//! anything: the field stays as `portaki.module.json` has it — absent, usually.

use std::path::Path;

use anyhow::{Context, Result};

/// Bounds of `schema/module.v1.json` — a host reads two or three lines, not a release note.
pub const MAX_LINES: usize = 5;
pub const MAX_CHARS: usize = 160;

/// The lines to publish: `--notes` as given, or the version's `CHANGELOG.md` section.
///
/// `--notes` is written for this purpose, so a line off bounds is refused. A `CHANGELOG.md` is
/// written for developers: its first lines are kept and long ones shortened.
pub fn lines(notes: &[String], module_root: &Path, version: &str) -> Result<Vec<String>> {
    if !notes.is_empty() {
        let notes: Vec<String> = notes.iter().map(|note| note.trim().to_string()).collect();
        if notes.len() > MAX_LINES {
            anyhow::bail!("--notes: at most {MAX_LINES} lines, got {}", notes.len());
        }
        if let Some(bad) = notes
            .iter()
            .find(|note| note.is_empty() || note.chars().count() > MAX_CHARS)
        {
            anyhow::bail!("--notes: each line holds 1 to {MAX_CHARS} characters — « {bad} »");
        }
        return Ok(notes);
    }
    let path = module_root.join("CHANGELOG.md");
    let markdown = match std::fs::read_to_string(&path) {
        Ok(markdown) => markdown,
        Err(failure) if failure.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(failure) => return Err(failure).with_context(|| format!("read {}", path.display())),
    };
    Ok(section(&markdown, version)
        .into_iter()
        .take(MAX_LINES)
        .map(|line| shorten(&line))
        .collect())
}

/// The top-level bullets under `## [version]` / `## version`, cleaned of release-please's
/// `**scope:**` prefix and trailing commit link.
fn section(markdown: &str, version: &str) -> Vec<String> {
    let mut inside = false;
    let mut bullets = Vec::new();
    for line in markdown.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            if inside {
                break;
            }
            inside = heading_version(heading) == version;
            continue;
        }
        let bullet = line.strip_prefix("* ").or_else(|| line.strip_prefix("- "));
        if let (true, Some(bullet)) = (inside, bullet) {
            let cleaned = clean(bullet);
            if !cleaned.is_empty() {
                bullets.push(cleaned);
            }
        }
    }
    bullets
}

/// `[0.6.1](https://…) (2026-09-24)` → `0.6.1`; `v1.2.0 - 2026-09-24` → `1.2.0`.
fn heading_version(heading: &str) -> &str {
    let heading = heading.trim();
    let token = match heading.strip_prefix('[') {
        Some(rest) => rest.split(']').next().unwrap_or(""),
        None => heading.split_whitespace().next().unwrap_or(""),
    };
    token.strip_prefix('v').unwrap_or(token)
}

/// `**access-guide:** add listing ([2b27528](https://…))` → `add listing`.
fn clean(bullet: &str) -> String {
    let mut text = bullet.trim();
    if let Some(rest) = text.strip_prefix("**") {
        if let Some((_, after)) = rest.split_once(":**") {
            text = after.trim_start();
        }
    }
    if text.ends_with(')') {
        if let Some(at) = text.rfind(" ([") {
            text = &text[..at];
        }
    }
    text.trim().to_string()
}

fn shorten(line: &str) -> String {
    if line.chars().count() <= MAX_CHARS {
        return line.to_string();
    }
    let kept: String = line.chars().take(MAX_CHARS - 1).collect();
    format!("{}…", kept.trim_end())
}

/// Writes `changelog` into the manifest, one `{ lang: line }` per line. No line, no change.
pub fn stamp(raw: &str, lines: &[String], lang: &str) -> Result<String> {
    if lines.is_empty() {
        return Ok(raw.to_string());
    }
    let mut manifest: serde_json::Value =
        serde_json::from_str(raw).context("parse publish manifest")?;
    let entries = lines
        .iter()
        .map(|line| serde_json::json!({ lang: line }))
        .collect();
    manifest
        .as_object_mut()
        .context("publish manifest is not a JSON object")?
        .insert("changelog".to_string(), serde_json::Value::Array(entries));
    serde_json::to_string_pretty(&manifest).context("serialise publish manifest")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    const RELEASE_PLEASE: &str = "# Changelog

## [0.6.1](https://github.com/o/r/compare/a-v0.6.0...a-v0.6.1) (2026-09-24)


### Bug Fixes

* **deps:** build against portaki-sdk 6.11 ([18cec5f](https://github.com/o/r/commit/18cec5f))

## [0.6.0](https://github.com/o/r/compare/a-v0.5.1...a-v0.6.0) (2026-09-23)


### Features

* **access-guide:** add catalogue listing ([2b27528](https://github.com/o/r/commit/2b27528))
* show the keypad code the day before arrival
  * a nested detail that is not a line of its own
";

    fn module(changelog: Option<&str>) -> tempfile::TempDir {
        let dir = tempdir().unwrap();
        if let Some(changelog) = changelog {
            fs::write(dir.path().join("CHANGELOG.md"), changelog).unwrap();
        }
        dir
    }

    #[test]
    fn the_version_section_is_found_and_cleaned() {
        let dir = module(Some(RELEASE_PLEASE));

        assert_eq!(
            lines(&[], dir.path(), "0.6.0").unwrap(),
            vec![
                "add catalogue listing",
                "show the keypad code the day before arrival"
            ]
        );
        assert_eq!(
            lines(&[], dir.path(), "0.6.1").unwrap(),
            vec!["build against portaki-sdk 6.11"]
        );
    }

    #[test]
    fn keep_a_changelog_headings_are_read_too() {
        let dir = module(Some(
            "## [Unreleased]\n- later\n## v1.2.0 - 2026-09-24\n- faster sync\n## 1.1.0\n- older\n",
        ));

        assert_eq!(
            lines(&[], dir.path(), "1.2.0").unwrap(),
            vec!["faster sync"]
        );
    }

    #[test]
    fn another_version_or_no_file_gives_nothing() {
        assert!(lines(&[], module(Some(RELEASE_PLEASE)).path(), "0.7.0")
            .unwrap()
            .is_empty());
        assert!(lines(&[], module(None).path(), "0.6.0").unwrap().is_empty());
    }

    #[test]
    fn a_long_section_is_cut_to_the_schema_bounds() {
        let many: String = (0..8).map(|i| format!("* line {i}\n")).collect();
        let long = format!("* {}\n", "x".repeat(300));
        let dir = module(Some(&format!("## 1.0.0\n{long}{many}")));

        let found = lines(&[], dir.path(), "1.0.0").unwrap();
        assert_eq!(found.len(), MAX_LINES);
        assert_eq!(found[0].chars().count(), MAX_CHARS);
        assert!(found[0].ends_with('…'));
    }

    #[test]
    fn notes_win_over_the_file() {
        let dir = module(Some(RELEASE_PLEASE));
        let notes = vec!["Nouveau : code clavier".to_string()];

        assert_eq!(lines(&notes, dir.path(), "0.6.0").unwrap(), notes);
    }

    #[test]
    fn notes_off_bounds_are_refused() {
        let dir = module(None);
        let six: Vec<String> = (0..6).map(|i| i.to_string()).collect();

        assert!(lines(&six, dir.path(), "1.0.0").is_err());
        assert!(lines(&["x".repeat(MAX_CHARS + 1)], dir.path(), "1.0.0").is_err());
        assert!(lines(&[" ".to_string()], dir.path(), "1.0.0").is_err());
    }

    #[test]
    fn no_line_leaves_the_manifest_alone() {
        let raw = r#"{"id":"a","changelog":[{"fr":"déclaré"}]}"#;

        assert_eq!(stamp(raw, &[], "en").unwrap(), raw);
    }

    fn schema_errors(manifest: &serde_json::Value) -> Vec<String> {
        let schema: serde_json::Value =
            serde_json::from_str(portaki_test_utils::conformance::MODULE_SCHEMA_V1).unwrap();
        jsonschema::validator_for(&schema)
            .unwrap()
            .iter_errors(manifest)
            .map(|error| error.to_string())
            .collect()
    }

    fn manifest_with(changelog: serde_json::Value) -> serde_json::Value {
        serde_json::json!({
            "id": "access-guide", "name": { "fr": "Accès" }, "description": { "fr": "d" },
            "version": "1.0.0", "author": { "name": "Portaki", "type": "official" },
            "icon": "key", "type": "official", "changelog": changelog
        })
    }

    #[test]
    fn the_stamped_manifest_follows_the_schema() {
        let raw = manifest_with(serde_json::Value::Null).to_string();
        let stamped = stamp(&raw, &["add listing".to_string()], "en").unwrap();
        let stamped: serde_json::Value = serde_json::from_str(&stamped).unwrap();

        assert_eq!(
            stamped["changelog"],
            serde_json::json!([{ "en": "add listing" }])
        );
        assert!(schema_errors(&stamped).is_empty());
    }

    #[test]
    fn the_schema_refuses_a_changelog_off_bounds() {
        let six: Vec<_> = (0..6).map(|_| serde_json::json!({ "fr": "x" })).collect();
        for changelog in [
            serde_json::json!(six),
            serde_json::json!([{ "fr": "x".repeat(MAX_CHARS + 1) }]),
            serde_json::json!([{ "fr": "" }]),
            serde_json::json!([{}]),
            serde_json::json!(["plain string"]),
        ] {
            assert!(
                !schema_errors(&manifest_with(changelog.clone())).is_empty(),
                "{changelog}"
            );
        }
        assert!(schema_errors(&manifest_with(serde_json::json!([
            { "fr": "Code clavier la veille", "en": "Keypad code the day before" }
        ])))
        .is_empty());
    }
}
