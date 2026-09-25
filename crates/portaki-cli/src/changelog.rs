//! `changelog` of the published manifest: what is new in this version, for hosts on an older one.
//!
//! One entry per line, every language side by side: `[{ "fr": "…", "en": "…" }, …]`. For each
//! language, `portaki publish --notes` wins (`--notes fr:"…"`, or unprefixed in `--notes-lang`);
//! otherwise the version's section of `CHANGELOG.<lang>.md`, and of `CHANGELOG.md` for the
//! default language (Keep a Changelog or release-please: `## [x.y.z]` or `## x.y.z`, then
//! bullets). Nothing said in any language: the field stays as `portaki.module.json` has it.
//!
//! A stable version stays pending at the registry until its changelog covers every language of
//! the module's listing — writing each `CHANGELOG.<lang>.md` is what keeps a CI green and live.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};

/// Bounds of `schema/module.v1.json` — a host reads two or three lines, not a release note.
pub const MAX_LINES: usize = 5;
pub const MAX_CHARS: usize = 160;

/// One changelog line, by language.
pub type Line = BTreeMap<String, String>;

/// `fr:Nouveau` → `("fr", "Nouveau")`. Two lowercase letters, a colon, then text right after:
/// `ui: fix` stays a line of its own, not a line in « ui ».
pub fn split_lang(value: &str) -> Option<(&str, &str)> {
    let (lang, text) = value.split_once(':')?;
    let tagged = lang.len() == 2
        && lang.chars().all(|c| c.is_ascii_lowercase())
        && text.chars().next().is_some_and(|c| !c.is_whitespace());
    tagged.then_some((lang, text))
}

/// A `[lang:]text` flag value, in `default_lang` when untagged.
pub fn tagged(value: &str, default_lang: &str) -> (String, String) {
    match split_lang(value) {
        Some((lang, text)) => (lang.to_string(), text.trim().to_string()),
        None => (default_lang.to_string(), value.trim().to_string()),
    }
}

/// 1 to [`MAX_CHARS`] characters, or the flag's name and the culprit.
pub fn check_text(flag: &str, text: &str) -> Result<()> {
    if text.is_empty() || text.chars().count() > MAX_CHARS {
        anyhow::bail!("{flag}: each text holds 1 to {MAX_CHARS} characters — « {text} »");
    }
    Ok(())
}

/// The lines to publish, one entry per line with every language that has one.
///
/// `--notes` is written for this purpose, so a line off bounds is refused. A `CHANGELOG.md` is
/// written for developers: its first lines are kept and long ones shortened.
pub fn lines(
    notes: &[String],
    default_lang: &str,
    module_root: &Path,
    version: &str,
) -> Result<Vec<Line>> {
    let mut by_lang: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for note in notes {
        let (lang, text) = tagged(note, default_lang);
        check_text("--notes", &text)?;
        by_lang.entry(lang).or_default().push(text);
    }
    if let Some((lang, many)) = by_lang.iter().find(|(_, lines)| lines.len() > MAX_LINES) {
        anyhow::bail!(
            "--notes: at most {MAX_LINES} lines per language, got {} in {lang}",
            many.len()
        );
    }
    for (lang, path) in changelog_files(module_root, default_lang)? {
        if by_lang.contains_key(&lang) {
            continue;
        }
        let markdown =
            std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        let found: Vec<String> = section(&markdown, version)
            .into_iter()
            .take(MAX_LINES)
            .map(|line| shorten(&line))
            .collect();
        if !found.is_empty() {
            by_lang.insert(lang, found);
        }
    }
    let count = by_lang.values().map(Vec::len).max().unwrap_or(0);
    Ok((0..count)
        .map(|index| {
            by_lang
                .iter()
                .filter_map(|(lang, lines)| Some((lang.clone(), lines.get(index)?.clone())))
                .collect()
        })
        .collect())
}

/// `CHANGELOG.md` in the default language, `CHANGELOG.<lang>.md` in its own.
fn changelog_files(
    module_root: &Path,
    default_lang: &str,
) -> Result<Vec<(String, std::path::PathBuf)>> {
    let entries = match std::fs::read_dir(module_root) {
        Ok(entries) => entries,
        Err(failure) if failure.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(failure) => {
            return Err(failure).with_context(|| format!("read {}", module_root.display()))
        }
    };
    let mut files = Vec::new();
    for entry in entries {
        let path = entry?.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let lang = match name
            .strip_prefix("CHANGELOG")
            .and_then(|rest| rest.strip_suffix(".md"))
        {
            Some("") => default_lang.to_string(),
            Some(rest) => match rest.strip_prefix('.') {
                Some(lang) if lang.len() == 2 && lang.chars().all(|c| c.is_ascii_lowercase()) => {
                    lang.to_string()
                }
                _ => continue,
            },
            None => continue,
        };
        files.push((lang, path));
    }
    Ok(files)
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

/// Writes `changelog` into the manifest, one entry per line. No line, no change.
pub fn stamp(raw: &str, lines: &[Line]) -> Result<String> {
    if lines.is_empty() {
        return Ok(raw.to_string());
    }
    let mut manifest: serde_json::Value =
        serde_json::from_str(raw).context("parse publish manifest")?;
    manifest
        .as_object_mut()
        .context("publish manifest is not a JSON object")?
        .insert("changelog".to_string(), serde_json::to_value(lines)?);
    serde_json::to_string_pretty(&manifest).context("serialise publish manifest")
}

/// What the registry is told beside the manifest, for this release only: why a permission is
/// added (`--permission-reason <permission>=[lang:]text`) and whether the host has to act
/// (`--host-action-required`, `--host-action [lang:]text`). Sent with the announcement, not
/// stamped into the manifest: the console can complete it later, and an older SDK schema would
/// refuse the extra keys.
pub fn release_notes(
    permission_reasons: &[String],
    host_action_required: bool,
    host_action: &[String],
    default_lang: &str,
) -> Result<serde_json::Value> {
    let mut reasons: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    for value in permission_reasons {
        let Some((permission, reason)) = value.split_once('=') else {
            anyhow::bail!("--permission-reason: expected <permission>=[lang:]text — « {value} »");
        };
        let permission = permission.trim();
        if !portaki_sdk::permission::is_known(permission) {
            anyhow::bail!("--permission-reason: unknown permission « {permission} »");
        }
        let (lang, text) = tagged(reason, default_lang);
        check_text("--permission-reason", &text)?;
        reasons
            .entry(permission.to_string())
            .or_default()
            .insert(lang, text);
    }
    let mut action = BTreeMap::new();
    for value in host_action {
        let (lang, text) = tagged(value, default_lang);
        check_text("--host-action", &text)?;
        action.insert(lang, text);
    }
    Ok(serde_json::json!({
        "permissionReasons": reasons,
        "hostActionRequired": host_action_required || !action.is_empty(),
        "hostAction": action,
    }))
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

    /// The English lines alone — what the single-language tests look at.
    fn en(notes: &[String], root: &Path, version: &str) -> Result<Vec<String>> {
        Ok(lines(notes, "en", root, version)?
            .into_iter()
            .filter_map(|line| line.get("en").cloned())
            .collect())
    }

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
            en(&[], dir.path(), "0.6.0").unwrap(),
            vec![
                "add catalogue listing",
                "show the keypad code the day before arrival"
            ]
        );
        assert_eq!(
            en(&[], dir.path(), "0.6.1").unwrap(),
            vec!["build against portaki-sdk 6.11"]
        );
    }

    #[test]
    fn keep_a_changelog_headings_are_read_too() {
        let dir = module(Some(
            "## [Unreleased]\n- later\n## v1.2.0 - 2026-09-24\n- faster sync\n## 1.1.0\n- older\n",
        ));

        assert_eq!(en(&[], dir.path(), "1.2.0").unwrap(), vec!["faster sync"]);
    }

    #[test]
    fn another_version_or_no_file_gives_nothing() {
        assert!(en(&[], module(Some(RELEASE_PLEASE)).path(), "0.7.0")
            .unwrap()
            .is_empty());
        assert!(en(&[], module(None).path(), "0.6.0").unwrap().is_empty());
    }

    #[test]
    fn a_long_section_is_cut_to_the_schema_bounds() {
        let many: String = (0..8).map(|i| format!("* line {i}\n")).collect();
        let long = format!("* {}\n", "x".repeat(300));
        let dir = module(Some(&format!("## 1.0.0\n{long}{many}")));

        let found = en(&[], dir.path(), "1.0.0").unwrap();
        assert_eq!(found.len(), MAX_LINES);
        assert_eq!(found[0].chars().count(), MAX_CHARS);
        assert!(found[0].ends_with('…'));
    }

    #[test]
    fn notes_win_over_the_file() {
        let dir = module(Some(RELEASE_PLEASE));
        let notes = vec!["Nouveau : code clavier".to_string()];

        assert_eq!(en(&notes, dir.path(), "0.6.0").unwrap(), notes);
    }

    #[test]
    fn notes_off_bounds_are_refused() {
        let dir = module(None);
        let six: Vec<String> = (0..6).map(|i| i.to_string()).collect();

        assert!(lines(&six, "en", dir.path(), "1.0.0").is_err());
        assert!(lines(&["x".repeat(MAX_CHARS + 1)], "en", dir.path(), "1.0.0").is_err());
        assert!(lines(&[" ".to_string()], "en", dir.path(), "1.0.0").is_err());
    }

    #[test]
    fn no_line_leaves_the_manifest_alone() {
        let raw = r#"{"id":"a","changelog":[{"fr":"déclaré"}]}"#;

        assert_eq!(stamp(raw, &[]).unwrap(), raw);
    }

    #[test]
    fn every_language_file_lands_on_the_same_lines() {
        let dir = module(Some("## 1.0.0\n- keypad code\n- faster sync\n"));
        fs::write(
            dir.path().join("CHANGELOG.fr.md"),
            "## 1.0.0\n- code clavier\n",
        )
        .unwrap();
        fs::write(dir.path().join("CHANGELOG.old.md"), "## 1.0.0\n- ignored\n").unwrap();

        assert_eq!(
            lines(&[], "en", dir.path(), "1.0.0").unwrap(),
            vec![
                Line::from([
                    ("en".into(), "keypad code".into()),
                    ("fr".into(), "code clavier".into())
                ]),
                Line::from([("en".into(), "faster sync".into())]),
            ]
        );
    }

    #[test]
    fn tagged_notes_win_for_their_language_only() {
        let dir = module(Some("## 1.0.0\n- from the file\n"));
        fs::write(
            dir.path().join("CHANGELOG.fr.md"),
            "## 1.0.0\n- du fichier\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("CHANGELOG.de.md"),
            "## 1.0.0\n- aus der Datei\n",
        )
        .unwrap();
        let notes = vec!["fr:Code clavier".to_string(), "ui: untagged".to_string()];

        let found = lines(&notes, "en", dir.path(), "1.0.0").unwrap();

        assert_eq!(found.len(), 1);
        assert_eq!(found[0]["fr"], "Code clavier");
        assert_eq!(found[0]["en"], "ui: untagged");
        assert_eq!(found[0]["de"], "aus der Datei");
    }

    #[test]
    fn more_than_five_notes_in_one_language_are_refused() {
        let dir = module(None);
        let mut notes: Vec<String> = (0..5).map(|i| format!("fr:ligne {i}")).collect();
        notes.push("en:one".into());
        assert!(lines(&notes, "en", dir.path(), "1.0.0").is_ok());
        notes.push("fr:sixième".into());
        assert!(lines(&notes, "en", dir.path(), "1.0.0").is_err());
    }

    #[test]
    fn release_notes_group_reasons_by_permission_and_language() {
        let notes = release_notes(
            &[
                "email=fr:Pour envoyer le code".into(),
                "email=To send the code".into(),
            ],
            false,
            &["fr:Reconnecter la serrure".into()],
            "en",
        )
        .unwrap();

        assert_eq!(
            notes,
            serde_json::json!({
                "permissionReasons": { "email": { "en": "To send the code", "fr": "Pour envoyer le code" } },
                "hostActionRequired": true,
                "hostAction": { "fr": "Reconnecter la serrure" },
            })
        );
    }

    #[test]
    fn release_notes_refuse_what_the_registry_would() {
        for bad in [
            "email",
            "nope=why",
            "email=",
            &format!("email={}", "x".repeat(MAX_CHARS + 1)),
        ] {
            assert!(
                release_notes(&[bad.to_string()], false, &[], "en").is_err(),
                "{bad}"
            );
        }
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
        let stamped = stamp(&raw, &[Line::from([("en".into(), "add listing".into())])]).unwrap();
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
