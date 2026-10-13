//! `changelog` of the published manifest: what is new in this version, for hosts on an older one.
//!
//! One entry per line, every language side by side: `[{ "fr": "…", "en": "…" }, …]`. For each
//! language, `portaki release --notes` wins (`--notes fr:"…"`, or unprefixed in `--notes-lang`);
//! otherwise the version's section of `CHANGELOG.<lang>.md`, and of `CHANGELOG.md` in English
//! (Keep a Changelog or release-please: `## [x.y.z]` or `## x.y.z`, then bullets). Nothing said
//! in any language: the field stays as `portaki.module.json` has it.
//!
//! Untagged `--notes` are in the listing's first language (`publishedLangs[0]` of
//! `listing.json`), else `fr` — the registry's fallback language.
//!
//! A line that reads like a commit message (`chore: bump deps`) is refused in `--notes`, written
//! by hand. In a `CHANGELOG*.md` — often generated — it is dropped with a warning: a language left
//! without a line then counts as absent, and the registry keeps a stable version as a draft.
//! Unless every dropped line is a dependency bump (`**deps:** …`, `bump <crate> to <version>`):
//! the version then gets a standard line in `fr` and `en` — and in `fr` too when
//! `CHANGELOG.fr.md` has no section while `CHANGELOG.md`'s bumps dependencies only. A real line
//! anywhere in the section, and no standard line is made up.
//!
//! In a monorepo, release-please writes into a module's `CHANGELOG.md` the commits scoped to other
//! modules (`**trails:** upload GPX…` in checklist's section): a line scoped to another module of
//! the repository is dropped; unscoped lines, and lines scoped to this module or to anything else
//! (`ui`, `guest`), are kept.
//!
//! A stable version stays pending at the registry until its changelog covers every language of
//! the module's listing — writing each `CHANGELOG.<lang>.md` is what keeps a CI green and live.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, Result};

/// Bounds of `schema/module.v1.json` and of the release-notes contract — a host reads two or
/// three lines, not a release note.
pub const MAX_LINES: usize = 5;
pub const MAX_CHARS: usize = 160;

/// One changelog line, by language.
pub type Line = BTreeMap<String, String>;

/// The registry's fallback language (`ModuleListing.FALLBACK_LANGUAGE`).
pub const FALLBACK_LANG: &str = "fr";

/// The language of `CHANGELOG.md`, unsuffixed — release-please writes it in English.
pub const UNSUFFIXED_LANG: &str = "en";

/// The changelog file that holds `lang`.
pub fn file_for(lang: &str) -> String {
    if lang == UNSUFFIXED_LANG {
        "CHANGELOG.md".to_string()
    } else {
        format!("CHANGELOG.{lang}.md")
    }
}

/// The language of untagged `--notes`: the listing's first, else `fr`.
pub fn default_lang(module_root: &Path) -> String {
    std::fs::read_to_string(module_root.join("listing.json"))
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .and_then(|listing| {
            listing
                .pointer("/publishedLangs/0")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        })
        .filter(|lang| lang.len() == 2 && lang.chars().all(|c| c.is_ascii_lowercase()))
        .unwrap_or_else(|| FALLBACK_LANG.to_string())
}

/// Synced copy of portaki-platform's `contracts/release-notes-rules.json`: the registry
/// (`ReleaseNotes`), the console's « Compléter la version » drawer and this CLI apply the same
/// rules. Replace the file when the platform's changes — the tests replay its examples.
const RULES: &str = include_str!("../contracts/release-notes-rules.json");

#[derive(serde::Deserialize)]
struct Rules {
    commit: CommitRules,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct CommitRules {
    conventional_types: Vec<String>,
    words: Vec<String>,
}

fn rules() -> &'static Rules {
    static PARSED: std::sync::OnceLock<Rules> = std::sync::OnceLock::new();
    PARSED.get_or_init(|| serde_json::from_str(RULES).expect("release-notes-rules.json is JSON"))
}

/// A commit message, not a line a host reads — the contract's `commit.rule`.
///
/// Trimmed and lowercased, it starts with (a) a conventional type, an optional `(scope)`, an
/// optional `!`, then `:` right away — « fix: x », « feat(ui)!: x »; or (b) one of the words,
/// followed by the end of the line or a character that is neither a letter nor a digit. A type
/// alone stays prose: « Style des cartes revu », « Chorégraphie des arrivées ».
pub fn looks_like_commit(line: &str) -> bool {
    let line = line.trim().to_lowercase();
    let rules = &rules().commit;
    let conventional = rules.conventional_types.iter().any(|kind| {
        let Some(mut rest) = line.strip_prefix(kind.as_str()) else {
            return false;
        };
        if let Some(scoped) = rest.strip_prefix('(') {
            match scoped.split_once(')') {
                Some((_, after)) => rest = after,
                None => return false,
            }
        }
        rest.strip_prefix('!').unwrap_or(rest).starts_with(':')
    });
    conventional
        || rules.words.iter().any(|word| {
            line.strip_prefix(word.as_str())
                .is_some_and(|rest| !rest.chars().next().is_some_and(char::is_alphanumeric))
        })
}

/// Refuses a commit-like `--notes` line: it was written by hand, for this purpose.
fn refuse_commit(text: &str) -> Result<()> {
    if looks_like_commit(text) {
        anyhow::bail!(
            "--notes: « {text} » reads like a commit message, not a line for hosts — say what \
             changes for them (e.g. « Le code clavier arrive la veille »); the registry keeps a \
             stable version with such a line invisible to hosts"
        );
    }
    Ok(())
}

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
        refuse_commit(&text)?;
        by_lang.entry(lang).or_default().push(text);
    }
    if let Some((lang, many)) = by_lang.iter().find(|(_, lines)| lines.len() > MAX_LINES) {
        anyhow::bail!(
            "--notes: at most {MAX_LINES} lines per language, got {} in {lang}",
            many.len()
        );
    }
    let others = other_modules(module_root);
    let mut with_section = BTreeSet::new();
    let mut deps_only = BTreeSet::new();
    for (lang, path) in changelog_files(module_root)? {
        if by_lang.contains_key(&lang) {
            continue;
        }
        let markdown =
            std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        let source = path.file_name().unwrap_or_default().to_string_lossy();
        let bullets = section(&markdown, version);
        if !bullets.is_empty() {
            with_section.insert(lang.clone());
        }
        let (foreign, ours): (Vec<_>, Vec<_>) = bullets
            .into_iter()
            .partition(|(scope, _)| scope.as_ref().is_some_and(|s| others.contains(s)));
        if !foreign.is_empty() {
            crate::ui::warn(format!(
                "{source} {version}: dropped {} line(s) scoped to other modules — {}",
                foreign.len(),
                foreign
                    .iter()
                    .map(|(scope, _)| scope.as_deref().unwrap_or_default())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        // Generated most of the time: a commit line is dropped from it, never a blocker.
        let (commits, kept): (Vec<_>, Vec<_>) = ours
            .into_iter()
            .partition(|(_, line)| looks_like_commit(line));
        if !commits.is_empty() {
            crate::ui::warn(format!(
                "{source} {version}: dropped {} line(s) that read like commit messages — {}",
                commits.len(),
                commits
                    .iter()
                    .map(|(_, line)| line.as_str())
                    .collect::<Vec<_>>()
                    .join(" · ")
            ));
        }
        let found: Vec<String> = kept
            .into_iter()
            .take(MAX_LINES)
            .map(|(_, line)| shorten(&line))
            .collect();
        if !found.is_empty() {
            by_lang.insert(lang, found);
        } else if foreign.is_empty()
            && !commits.is_empty()
            && commits
                .iter()
                .all(|(scope, line)| is_deps_bump(scope.as_deref(), line))
        {
            deps_only.insert(lang.clone());
            if let Some(standard) = deps_only_line(&lang) {
                crate::ui::warn(format!(
                    "{source} {version}: dependency bumps only — standard line used: « {standard} »"
                ));
                by_lang.insert(lang, vec![standard.to_string()]);
            }
        }
    }
    // A release-please patch for an SDK bump: nobody writes CHANGELOG.fr.md for it, and the
    // registry's fallback language is the one a stable version cannot go without.
    if deps_only.contains(UNSUFFIXED_LANG)
        && !by_lang.contains_key(FALLBACK_LANG)
        && !with_section.contains(FALLBACK_LANG)
    {
        if let Some(standard) = deps_only_line(FALLBACK_LANG) {
            crate::ui::warn(format!(
                "{} {version}: no section, CHANGELOG.md bumps dependencies only — standard line \
                 used: « {standard} »",
                file_for(FALLBACK_LANG)
            ));
            by_lang.insert(FALLBACK_LANG.to_string(), vec![standard.to_string()]);
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

/// A dependency bump, release-please's `**deps:** …` or a bare `bump <crate> to <version>`.
fn is_deps_bump(scope: Option<&str>, line: &str) -> bool {
    if scope == Some("deps") {
        return true;
    }
    let line = line.to_lowercase();
    match line.split_whitespace().collect::<Vec<_>>()[..] {
        ["bump", krate, "to", version] => {
            let version = version.strip_prefix('v').unwrap_or(version);
            !krate.is_empty()
                && version.starts_with(|c: char| c.is_ascii_digit())
                && version
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || ".-+".contains(c))
        }
        _ => false,
    }
}

/// What a host reads for a version that only bumps dependencies — fr and en only.
fn deps_only_line(lang: &str) -> Option<&'static str> {
    match lang {
        "fr" => Some(
            "Le module passe à la dernière base technique de Portaki ; rien ne change pour \
             l'hôte ni pour le voyageur.",
        ),
        "en" => {
            Some("Updated to Portaki's latest technical base; nothing changes for hosts or guests.")
        }
        _ => None,
    }
}

/// The ids of the repository's other modules — none outside a monorepo.
fn other_modules(module_root: &Path) -> BTreeSet<String> {
    let own = std::fs::canonicalize(module_root).ok();
    crate::workspace::members(module_root)
        .into_iter()
        .filter(|member| std::fs::canonicalize(&member.root).ok() != own)
        .map(|member| member.id)
        .collect()
}

/// `CHANGELOG.md` in English, `CHANGELOG.<lang>.md` in its own.
fn changelog_files(module_root: &Path) -> Result<Vec<(String, std::path::PathBuf)>> {
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
            Some("") => UNSUFFIXED_LANG.to_string(),
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
/// `**scope:**` prefix (returned beside the text) and trailing commit link.
fn section(markdown: &str, version: &str) -> Vec<(Option<String>, String)> {
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
            let (scope, cleaned) = clean(bullet);
            if !cleaned.is_empty() {
                bullets.push((scope.map(str::to_string), cleaned));
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

/// `**access-guide:** add listing ([2b27528](https://…))` → `(Some("access-guide"), "add listing")`.
fn clean(bullet: &str) -> (Option<&str>, String) {
    let mut text = bullet.trim();
    let mut scope = None;
    if let Some(rest) = text.strip_prefix("**") {
        if let Some((found, after)) = rest.split_once(":**") {
            scope = Some(found.trim());
            text = after.trim_start();
        }
    }
    if text.ends_with(')') {
        if let Some(at) = text.rfind(" ([") {
            text = &text[..at];
        }
    }
    (scope, text.trim().to_string())
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

/// The module's permission reasons, written once beside its sources so a CI passes them:
/// `{ "<permission>": { "fr": "…", "en": "…" } }`, a permission or a `reveal:<field>:<place>` key.
pub const REASONS_FILE: &str = "permission-reasons.json";

/// Reasons by permission, then by language.
pub type Reasons = BTreeMap<String, BTreeMap<String, String>>;

/// A key `--permission-reason` and [`REASONS_FILE`] accept: a manifest permission, or a revealed
/// secret's `reveal:<field>:<place>`.
fn is_reason_key(key: &str) -> bool {
    portaki_sdk::permission::is_known(key) || portaki_sdk::permission::is_reveal(key)
}

/// [`REASONS_FILE`] at `module_root`, checked like `--permission-reason`; empty when absent.
///
/// With `wanted`, only its keys are kept — the others are left out with a warning: a reason for a
/// permission this version does not declare (or, completing a version, does not add) is noise.
pub fn file_reasons(module_root: &Path, wanted: Option<&BTreeSet<String>>) -> Result<Reasons> {
    let path = module_root.join(REASONS_FILE);
    let raw = match std::fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(failure) if failure.kind() == std::io::ErrorKind::NotFound => return Ok(Reasons::new()),
        Err(failure) => return Err(failure).with_context(|| format!("read {}", path.display())),
    };
    let parsed: Reasons = serde_json::from_str(&raw).with_context(|| {
        format!(
            "{REASONS_FILE}: expected {{ \"<permission>\": {{ \"fr\": \"…\", \"en\": \"…\" }} }}"
        )
    })?;
    let mut kept = Reasons::new();
    let mut ignored = Vec::new();
    for (permission, by_lang) in parsed {
        if !is_reason_key(&permission) {
            anyhow::bail!("{REASONS_FILE}: unknown permission « {permission} »");
        }
        for (lang, text) in &by_lang {
            if !(lang.len() == 2 && lang.chars().all(|c| c.is_ascii_lowercase())) {
                anyhow::bail!("{REASONS_FILE}: « {permission} » — unknown language « {lang} »");
            }
            check_text(REASONS_FILE, text.trim())?;
        }
        if wanted.is_some_and(|wanted| !wanted.contains(&permission)) {
            ignored.push(permission);
            continue;
        }
        let by_lang = by_lang
            .into_iter()
            .map(|(lang, text)| (lang, text.trim().to_string()))
            .collect();
        kept.insert(permission, by_lang);
    }
    if !ignored.is_empty() {
        crate::ui::warn(format!(
            "{REASONS_FILE}: not sent, this version does not ask for them — {}",
            ignored.join(", ")
        ));
    }
    Ok(kept)
}

/// What a built manifest declares that a reason may be asked for: its `permissions`, and a
/// `reveal:<field>:<place>` key per place of each revealed secret — the field's key, or
/// `<list>[].<sub>` for each `item.secret` sub-key of a structured list (`SecretReveal.declaredIn`).
pub fn declared(manifest: &serde_json::Value) -> BTreeSet<String> {
    let strings = |value: &serde_json::Value| -> Vec<String> {
        value
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
            .filter(|text| !text.trim().is_empty())
            .map(str::to_string)
            .collect()
    };
    let mut found: BTreeSet<String> = strings(&manifest["permissions"]).into_iter().collect();
    for field in manifest["config"]["fields"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let Some(key) = field["key"].as_str().filter(|key| !key.trim().is_empty()) else {
            continue;
        };
        let secrets = match field["type"].as_str() {
            Some("secret") => vec![key.to_string()],
            Some("structured") => strings(&field["item"]["secret"])
                .into_iter()
                .map(|sub| format!("{key}[].{sub}"))
                .collect(),
            _ => Vec::new(),
        };
        for place in strings(&field["reveal"])
            .into_iter()
            .filter(|place| portaki_sdk::permission::REVEAL_PLACES.contains(&place.as_str()))
        {
            for secret in &secrets {
                found.insert(format!(
                    "{}{secret}:{place}",
                    portaki_sdk::permission::REVEAL_PREFIX
                ));
            }
        }
    }
    found
}

/// What the registry is told beside the manifest, for this release only: why a permission is
/// added (`file`, read from [`REASONS_FILE`], then `--permission-reason <permission>=[lang:]text`
/// over it per permission and language) and whether the host has to act
/// (`--host-action-required`, `--host-action [lang:]text`). Sent with the announcement, not
/// stamped into the manifest: the console can complete it later, and an older SDK schema would
/// refuse the extra keys.
pub fn release_notes(
    permission_reasons: &[String],
    host_action_required: bool,
    host_action: &[String],
    default_lang: &str,
    file: Reasons,
) -> Result<serde_json::Value> {
    let mut reasons = file;
    for value in permission_reasons {
        let Some((permission, reason)) = value.split_once('=') else {
            anyhow::bail!("--permission-reason: expected <permission>=[lang:]text — « {value} »");
        };
        let permission = permission.trim();
        if !is_reason_key(permission) {
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

    /// The contract's examples, replayed as the registry replays them.
    #[test]
    fn the_release_notes_contract_is_applied_as_written() {
        let contract: serde_json::Value = serde_json::from_str(RULES).unwrap();
        assert_eq!(contract["maxLines"], MAX_LINES);
        assert_eq!(contract["maxChars"], MAX_CHARS);
        let examples = |kind: &str| -> Vec<String> {
            contract["commit"]["examples"][kind]
                .as_array()
                .unwrap()
                .iter()
                .map(|example| example.as_str().unwrap().to_string())
                .collect()
        };
        for commit in examples("commit") {
            assert!(looks_like_commit(&commit), "a commit: {commit}");
        }
        for prose in examples("prose") {
            assert!(!looks_like_commit(&prose), "prose: {prose}");
        }
        assert!(!examples("prose").is_empty());
    }

    #[test]
    fn a_commit_message_is_refused_in_notes_and_dropped_from_files() {
        for commit in [
            "chore: bump deps",
            "  Bump portaki-sdk",
            "fix(ui): wrap",
            "feat(x): y",
            "Fix: typo",
            "FEAT: new",
            "refactor the view",
            "deps: bump portaki-sdk to 8.8.1",
        ] {
            assert!(looks_like_commit(commit), "{commit}");
            let dir = module(None);
            assert!(lines(&[commit.to_string()], "en", dir.path(), "1.0.0").is_err());
            let dir = module(Some(&format!("## 1.0.0\n- {commit}\n- keypad code\n")));
            assert_eq!(en(&[], dir.path(), "1.0.0").unwrap(), vec!["keypad code"]);
        }
        for fine in [
            "Fixes the keypad code",
            "Featured on arrival",
            "Code clavier",
        ] {
            assert!(!looks_like_commit(fine), "{fine}");
        }
    }

    /// Commit lines that are not all dependency bumps leave the language absent, not an error.
    #[test]
    fn a_file_left_with_commits_only_counts_as_absent() {
        for section in [
            "* **deps:** bump portaki-sdk to 8.8.1 ([474417d](https://x))\n* chore: release 1.2\n",
            "* chore: release 1.2\n",
            "* build(deps): bump serde\n",
        ] {
            let dir = module(Some(&format!(
                "## [0.8.2](https://x) (2026-09-26)\n\n### Bug Fixes\n\n{section}"
            )));
            assert!(
                lines(&[], "fr", dir.path(), "0.8.2").unwrap().is_empty(),
                "{section}"
            );
        }
    }

    /// nuki 1.8.1 in portaki-modules' release-please PR: an SDK bump, nothing else.
    const DEPS_ONLY: &str = "# Changelog

## [1.8.1](https://github.com/PortakiApp/portaki-modules/compare/nuki-v1.8.0...nuki-v1.8.1) (2026-10-13)


### Bug Fixes

* **deps:** bump portaki-sdk to 10.10.0 ([f9906b0](https://github.com/PortakiApp/portaki-modules/commit/f9906b0045b44a3f033f1feac24ef7b0ba7f4954))

## [1.8.0](https://github.com/PortakiApp/portaki-modules/compare/nuki-v1.7.0...nuki-v1.8.0) (2026-10-12)


### Features

* **nuki:** show the lock's battery ([1a2b3c4](https://github.com/PortakiApp/portaki-modules/commit/1a2b3c4))
";

    fn standard() -> Vec<Line> {
        vec![Line::from([
            ("en".into(), deps_only_line("en").unwrap().into()),
            ("fr".into(), deps_only_line("fr").unwrap().into()),
        ])]
    }

    #[test]
    fn a_deps_only_version_gets_the_standard_line_in_french_and_english() {
        // No CHANGELOG.fr.md at all, then one without a section for 1.8.1.
        let dir = module(Some(DEPS_ONLY));
        assert_eq!(lines(&[], "fr", dir.path(), "1.8.1").unwrap(), standard());
        fs::write(
            dir.path().join("CHANGELOG.fr.md"),
            "# Changelog\n\n## 1.8.0\n\n* La batterie de la serrure s'affiche\n",
        )
        .unwrap();
        assert_eq!(lines(&[], "fr", dir.path(), "1.8.1").unwrap(), standard());
        // The version before is real work: no standard line.
        assert_eq!(
            en(&[], dir.path(), "1.8.0").unwrap(),
            vec!["show the lock's battery"]
        );
        for lang in ["fr", "en"] {
            let line = deps_only_line(lang).unwrap();
            assert!(
                check_text("x", line).is_ok() && !looks_like_commit(line),
                "{line}"
            );
        }
    }

    #[test]
    fn both_forms_of_a_dependency_bump_are_recognised() {
        for section in [
            "- bump portaki-sdk to 10.10.0\n",
            "- Bump portaki-sdk to v10.10.0\n",
            "* **deps:** bump deps ([1](https://x))\n",
            "* **deps:** bump portaki-sdk to 10.10.0\n* **deps:** bump serde to 1.0.200-rc.1\n",
        ] {
            let dir = module(Some(&format!("## 1.0.1\n{section}")));
            assert_eq!(
                lines(&[], "fr", dir.path(), "1.0.1").unwrap(),
                standard(),
                "{section}"
            );
        }
        for (scope, line, bump) in [
            (Some("deps"), "bump deps", true),
            (None, "bump portaki-sdk to 10.10.0", true),
            (None, "bump portaki-sdk", false),
            (None, "bump portaki-sdk to latest", false),
            (None, "bump portaki-sdk to 10.10.0 and serde", false),
            (None, "build(deps): bump serde", false),
            (Some("ui"), "bump deps", false),
        ] {
            assert_eq!(is_deps_bump(scope, line), bump, "{scope:?} {line}");
        }
    }

    #[test]
    fn a_real_line_is_never_replaced_by_the_standard_one() {
        // A real English line beside the bump: kept alone, French stays absent.
        let dir = module(Some(
            "## 1.0.1\n* **deps:** bump portaki-sdk to 10.10.0\n* keypad code\n",
        ));
        assert_eq!(
            lines(&[], "fr", dir.path(), "1.0.1").unwrap(),
            vec![Line::from([("en".into(), "keypad code".into())])]
        );
        // A French section of its own wins over the standard line.
        let dir = module(Some(DEPS_ONLY));
        fs::write(
            dir.path().join("CHANGELOG.fr.md"),
            "## 1.8.1\n* Code clavier la veille\n",
        )
        .unwrap();
        assert_eq!(
            lines(&[], "fr", dir.path(), "1.8.1").unwrap(),
            vec![Line::from([
                ("en".into(), deps_only_line("en").unwrap().into()),
                ("fr".into(), "Code clavier la veille".into()),
            ])]
        );
        // A French section of commit lines only, not bumps: absent, not made up from English.
        fs::write(
            dir.path().join("CHANGELOG.fr.md"),
            "## 1.8.1\n* chore: release\n",
        )
        .unwrap();
        assert_eq!(
            lines(&[], "fr", dir.path(), "1.8.1").unwrap(),
            vec![Line::from([(
                "en".into(),
                deps_only_line("en").unwrap().into()
            )])]
        );
        // `--notes` win, and a language without a standard line stays absent.
        fs::write(dir.path().join("CHANGELOG.fr.md"), "").unwrap();
        fs::write(
            dir.path().join("CHANGELOG.de.md"),
            "## 1.8.1\n* **deps:** bump portaki-sdk to 10.10.0\n",
        )
        .unwrap();
        let found = lines(&["fr:Code clavier".into()], "en", dir.path(), "1.8.1").unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0]["fr"], "Code clavier");
        assert!(!found[0].contains_key("de"));
    }

    #[test]
    fn a_bump_beside_another_modules_line_gets_no_standard_line() {
        let repo = tempdir().unwrap();
        let nuki = monorepo_member(repo.path(), "nuki");
        monorepo_member(repo.path(), "trails");
        fs::write(
            nuki.join("CHANGELOG.md"),
            "## 1.8.1\n* **trails:** upload GPX\n* **deps:** bump portaki-sdk to 10.10.0\n",
        )
        .unwrap();

        assert!(lines(&[], "fr", &nuki, "1.8.1").unwrap().is_empty());
    }

    /// `CHANGELOG.md` stays English whatever the notes language: no English text tagged `fr`.
    #[test]
    fn the_unsuffixed_changelog_is_english() {
        let dir = module(Some("## 1.0.0\n- keypad code\n"));

        let found = lines(&[], "fr", dir.path(), "1.0.0").unwrap();
        assert_eq!(
            found,
            vec![Line::from([("en".into(), "keypad code".into())])]
        );
        assert_eq!(file_for("en"), "CHANGELOG.md");
        assert_eq!(file_for("fr"), "CHANGELOG.fr.md");
    }

    #[test]
    fn the_default_language_is_the_listings_first_else_french() {
        let dir = module(None);
        assert_eq!(default_lang(dir.path()), "fr");
        fs::write(
            dir.path().join("listing.json"),
            r#"{"publishedLangs":["en","fr"]}"#,
        )
        .unwrap();
        assert_eq!(default_lang(dir.path()), "en");
        fs::write(dir.path().join("listing.json"), r#"{"publishedLangs":[]}"#).unwrap();
        assert_eq!(default_lang(dir.path()), "fr");
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
            Reasons::new(),
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
                release_notes(&[bad.to_string()], false, &[], "en", Reasons::new()).is_err(),
                "{bad}"
            );
        }
    }

    #[test]
    fn a_revealed_secret_is_justified_by_its_reveal_key() {
        let notes = release_notes(
            &[
                "reveal:networks[].password:guest_stay=fr:Le mot de passe du Wi-Fi".into(),
                "reveal:bin_room_code:guest_pre_arrival=en:The bin room code".into(),
            ],
            false,
            &[],
            "en",
            Reasons::new(),
        )
        .unwrap();

        assert_eq!(
            notes["permissionReasons"]["reveal:networks[].password:guest_stay"],
            serde_json::json!({ "fr": "Le mot de passe du Wi-Fi" })
        );
        assert!(release_notes(
            &["reveal:code:host=x".into()],
            false,
            &[],
            "en",
            Reasons::new()
        )
        .is_err());
    }

    fn reasons_file(json: &str) -> tempfile::TempDir {
        let dir = module(None);
        fs::write(dir.path().join(REASONS_FILE), json).unwrap();
        dir
    }

    /// A CI passes no flag: the file says it once, the flags still win per permission and language.
    #[test]
    fn the_reasons_file_is_read_and_the_flags_override_it() {
        let dir = reasons_file(
            r#"{ "email": { "fr": " Pour envoyer le code ", "en": "To send the code" },
                 "reveal:code:guest_stay": { "fr": "Le code", "en": "The code" } }"#,
        );
        let wanted = BTreeSet::from(["email".to_string()]);

        let file = file_reasons(dir.path(), Some(&wanted)).unwrap();
        let notes = release_notes(&["email=en:To send it".into()], false, &[], "fr", file).unwrap();

        assert_eq!(
            notes["permissionReasons"],
            serde_json::json!({ "email": { "fr": "Pour envoyer le code", "en": "To send it" } })
        );
        assert_eq!(file_reasons(dir.path(), None).unwrap().len(), 2);
        assert!(file_reasons(module(None).path(), None).unwrap().is_empty());
    }

    #[test]
    fn the_reasons_file_is_checked_like_the_flag() {
        for bad in [
            r#"{ "nope": { "fr": "x" } }"#,
            r#"{ "email": { "fr": "" } }"#,
            r#"{ "email": { "français": "x" } }"#,
            r#"{ "email": "x" }"#,
            "not json",
        ] {
            assert!(
                file_reasons(reasons_file(bad).path(), None).is_err(),
                "{bad}"
            );
        }
        let long = format!(
            r#"{{ "email": {{ "fr": "{}" }} }}"#,
            "x".repeat(MAX_CHARS + 1)
        );
        assert!(file_reasons(reasons_file(&long).path(), None).is_err());
    }

    #[test]
    fn a_manifest_declares_its_permissions_and_each_revealed_secret() {
        let manifest = serde_json::json!({
            "permissions": ["kv", "email"],
            "config": { "fields": [
                { "key": "bin_room_code", "type": "secret", "reveal": ["guest_pre_arrival", "guest_stay"] },
                { "key": "networks", "type": "structured", "item": { "secret": ["password"] },
                  "reveal": ["guest_stay"] },
                { "key": "hidden", "type": "secret" },
                { "key": "ssid", "type": "text", "reveal": ["guest_stay"] },
            ] }
        });

        assert_eq!(
            declared(&manifest),
            BTreeSet::from(
                [
                    "email",
                    "kv",
                    "reveal:bin_room_code:guest_pre_arrival",
                    "reveal:bin_room_code:guest_stay",
                    "reveal:networks[].password:guest_stay",
                ]
                .map(String::from)
            )
        );
    }

    /// checklist 1.9.0 in portaki-modules: release-please filed other modules' commits under it.
    const MONOREPO_SECTION: &str = "# Changelog

## [1.9.0](https://github.com/PortakiApp/portaki-modules/compare/checklist-v1.8.0...checklist-v1.9.0) (2026-10-12)


### Features

* **appliances:** settings page and devices table ([5843ee5](https://github.com/PortakiApp/portaki-modules/commit/5843ee5))
* **checklist:** redesign host settings ([fa8e81b](https://github.com/PortakiApp/portaki-modules/commit/fa8e81b))
* **trails:** upload GPX as a file, not an image ([9b3db9e](https://github.com/PortakiApp/portaki-modules/commit/9b3db9e))
* **guest:** scope the prep-card routes per module ([09bb90c](https://github.com/PortakiApp/portaki-modules/commit/09bb90c))
* show steps in order


### Bug Fixes

* **deps:** bump portaki-sdk to 10.4.0 ([909824e](https://github.com/PortakiApp/portaki-modules/commit/909824e))
";

    fn monorepo_member(repo: &Path, id: &str) -> std::path::PathBuf {
        let root = repo.join("modules").join(id);
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            format!(
                "[package]\nname = \"{id}\"\nversion = \"1.0.0\"\n\n[dependencies]\nportaki-sdk = \"10\"\n"
            ),
        )
        .unwrap();
        root
    }

    #[test]
    fn lines_scoped_to_another_module_of_the_repository_are_dropped() {
        let repo = tempdir().unwrap();
        let checklist = monorepo_member(repo.path(), "checklist");
        monorepo_member(repo.path(), "appliances");
        monorepo_member(repo.path(), "trails");
        fs::write(checklist.join("CHANGELOG.md"), MONOREPO_SECTION).unwrap();
        assert_eq!(
            crate::workspace::members(&checklist).len(),
            3,
            "the fixture is a monorepo"
        );

        assert_eq!(
            en(&[], &checklist, "1.9.0").unwrap(),
            vec![
                "redesign host settings",
                "scope the prep-card routes per module",
                "show steps in order"
            ]
        );
        // A single-module repository has no other module: every scope stays.
        let alone = module(Some(MONOREPO_SECTION));
        assert_eq!(en(&[], alone.path(), "1.9.0").unwrap().len(), 5);
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
