//! `portaki release status <v>` et `portaki release notes <v>` — une version publiée, telle que
//! la page Versions de l'espace développeur la montre, et le tiroir « Compléter la version ».
//!
//! Routes : `GET /dev/v1/modules/{id}/versions` pour trouver le digest de la version, puis
//! `GET|PUT /dev/v1/publications/{digest}/release`. Le registre revalide tout au `PUT` : ce qui
//! manque encore revient en 409, rien n'est écrit.

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use reqwest::Method;
use serde_json::{json, Value};

use crate::api::Platform;
use crate::workspace::ModuleArgs;
use crate::{changelog, ui};

#[derive(Debug, Subcommand)]
pub enum ReleaseAction {
    /// Where a published version stands: draft or available, signature, review, what is missing.
    Status(StatusArgs),
    /// Read a version's release notes — and complete them, from flags or CHANGELOG files.
    Notes(NotesArgs),
}

#[derive(Debug, Parser)]
/// Arguments for `portaki release status`.
pub struct StatusArgs {
    /// The version, e.g. `1.2.0`.
    pub version: String,
    /// The channel, when the version exists on both.
    #[arg(long, value_parser = ["preview", "stable"])]
    pub channel: Option<String>,
    #[command(flatten)]
    pub modules: ModuleArgs,
}

#[derive(Debug, Parser)]
/// Arguments for `portaki release notes`.
pub struct NotesArgs {
    /// The version, e.g. `1.2.0`.
    pub version: String,
    /// The channel, when the version exists on both.
    #[arg(long, value_parser = ["preview", "stable"])]
    pub channel: Option<String>,
    /// Send the notes to the registry: the changelog from `--notes`, then from `CHANGELOG*.md`.
    /// Implied by any of the flags below.
    #[arg(long)]
    pub complete: bool,
    /// A changelog line, `[lang:]text` — replaces the language's lines.
    #[arg(long = "notes", value_name = "[LANG:]LINE")]
    pub notes: Vec<String>,
    /// Why a permission is added, `<permission>=[lang:]text`.
    #[arg(long = "permission-reason", value_name = "PERMISSION=[LANG:]TEXT")]
    pub permission_reasons: Vec<String>,
    /// The host has to act after updating.
    #[arg(long)]
    pub host_action_required: bool,
    /// What the host has to do, `[lang:]text`.
    #[arg(long = "host-action", value_name = "[LANG:]TEXT")]
    pub host_action: Vec<String>,
    #[command(flatten)]
    pub modules: ModuleArgs,
}

impl NotesArgs {
    fn completes(&self) -> bool {
        self.complete
            || !self.notes.is_empty()
            || !self.permission_reasons.is_empty()
            || self.host_action_required
            || !self.host_action.is_empty()
    }
}

/// Runs `portaki release status|notes`.
pub async fn run(action: ReleaseAction) -> Result<()> {
    match action {
        ReleaseAction::Status(args) => status(args).await,
        ReleaseAction::Notes(args) => notes(args).await,
    }
}

async fn status(args: StatusArgs) -> Result<()> {
    ui::header(
        "portaki release status",
        "Where a published version stands — and what it still needs.",
    );
    let member = args.modules.one("release status")?;
    let mut platform = open()?;
    let (entry, release) = find(
        &mut platform,
        &member.id,
        &args.version,
        args.channel.as_deref(),
    )
    .await?;
    let next = next_for(&release, &args.version, &args.modules);

    if ui::json() {
        ui::emit(&json!({
            "schemaVersion": 1,
            "module": member.id,
            "version": args.version,
            "channel": release["channel"],
            "digest": release["digest"],
            "state": release["releaseState"],
            "review": entry["status"],
            "yanked": entry["yanked"],
            "signature": release["supplyChain"]["signature"].as_str().unwrap_or("unsigned"),
            "signatureSource": release["supplyChain"]["signatureSource"],
            "missing": release["missing"],
            "next": { "command": next.0, "reason": next.1 },
        }));
        return Ok(());
    }

    let text = |value: &Value| value.as_str().unwrap_or_default().to_string();
    ui::field(
        "version",
        format!("{} · {}", args.version, text(&release["channel"])),
    );
    ui::field("digest", text(&release["digest"]));
    ui::field("state", text(&release["releaseState"]));
    ui::field("review", text(&entry["status"]));
    ui::field("signature", signature_line(&release["supplyChain"]));
    if entry["yanked"].as_bool().unwrap_or(false) {
        ui::warn("yanked — hosts can no longer install it");
    }
    print_missing(&release["missing"]);
    ui::next(&[(next.0.as_deref().unwrap_or("—"), &next.1)]);
    ui::blank();
    Ok(())
}

async fn notes(args: NotesArgs) -> Result<()> {
    ui::header(
        "portaki release notes",
        "A version's release notes, as hosts read them — completed here, checked by the registry.",
    );
    let member = args.modules.one("release notes")?;
    let mut platform = open()?;
    let (_, mut release) = find(
        &mut platform,
        &member.id,
        &args.version,
        args.channel.as_deref(),
    )
    .await?;

    if args.completes() {
        let body = merged_notes(&release["notes"], &args, &member.root)?;
        let sending = ui::step("sending the notes to the registry");
        let path = format!("/dev/v1/publications/{}/release", text(&release["digest"]));
        let (status, answer) = platform.send(Method::PUT, &path, Some(&body)).await?;
        match status {
            200..=299 => {
                sending.done(format!(
                    "{} is {}",
                    args.version,
                    text(&answer["releaseState"])
                ));
                release = answer;
            }
            409 => {
                sending.abandon();
                // Refusé sans rien écrire : ce qui manque est dans le refus, à dire tel quel.
                print_missing(&answer["missing"]);
                anyhow::bail!(
                    "the registry kept {} as a draft — {}",
                    args.version,
                    answer["message"]
                        .as_str()
                        .unwrap_or("notes still incomplete")
                );
            }
            _ => {
                sending.abandon();
                anyhow::bail!(
                    "{}",
                    crate::http::refused(
                        &format!("{}{path}", platform.base),
                        status,
                        &answer.to_string()
                    )
                );
            }
        }
    }

    if ui::json() {
        ui::emit(&json!({
            "schemaVersion": 1,
            "module": member.id,
            "version": args.version,
            "digest": release["digest"],
            "state": release["releaseState"],
            "langs": release["langs"],
            "added": release["added"],
            "notes": release["notes"],
            "missing": release["missing"],
        }));
        return Ok(());
    }

    let notes = &release["notes"];
    let langs: Vec<String> = release["langs"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|lang| lang.as_str().map(str::to_string))
        .collect();
    ui::field("state", text(&release["releaseState"]));
    ui::field("languages", langs.join(", "));
    for lang in &langs {
        ui::section(&format!("changelog · {lang}"));
        let lines: Vec<&str> = notes["changelog"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|line| line[lang.as_str()].as_str())
            .collect();
        if lines.is_empty() {
            ui::skipped("no line");
        }
        for line in lines {
            ui::detail(format!("- {line}"));
        }
    }
    if let Some(reasons) = notes["permissionReasons"]
        .as_object()
        .filter(|r| !r.is_empty())
    {
        ui::section("permission reasons");
        for (permission, by_lang) in reasons {
            for (lang, reason) in by_lang.as_object().into_iter().flatten() {
                ui::field(
                    permission,
                    format!("{lang}: {}", reason.as_str().unwrap_or_default()),
                );
            }
        }
    }
    if notes["hostActionRequired"].as_bool().unwrap_or(false) {
        ui::section("host action");
        for (lang, action) in notes["hostAction"].as_object().into_iter().flatten() {
            ui::field(lang, action.as_str().unwrap_or_default());
        }
    }
    print_missing(&release["missing"]);
    let next = next_for(&release, &args.version, &args.modules);
    ui::next(&[(next.0.as_deref().unwrap_or("—"), &next.1)]);
    ui::blank();
    Ok(())
}

fn open() -> Result<Platform> {
    Platform::open(&crate::profile::api_url(None))
}

fn text(value: &Value) -> String {
    value.as_str().unwrap_or_default().to_string()
}

/// L'entrée de la liste des versions, et sa page de release.
async fn find(
    platform: &mut Platform,
    id: &str,
    version: &str,
    channel: Option<&str>,
) -> Result<(Value, Value)> {
    let versions = platform
        .get(&format!("/dev/v1/modules/{id}/versions"))
        .await?
        .unwrap_or(Value::Array(Vec::new()));
    let entry = pick(&versions, version, channel).with_context(|| {
        format!("{id} {version} is not published — portaki status lists the latest version")
    })?;
    let digest = text(&entry["digest"]);
    let release = platform
        .get(&format!("/dev/v1/publications/{digest}/release"))
        .await?
        .with_context(|| format!("no release page for {digest}"))?;
    Ok((entry, release))
}

/// La version demandée ; la stable d'abord quand elle existe sur les deux canaux.
fn pick(versions: &Value, version: &str, channel: Option<&str>) -> Option<Value> {
    let matching: Vec<&Value> = versions
        .as_array()?
        .iter()
        .filter(|entry| entry["version"] == version)
        .filter(|entry| channel.map_or(true, |channel| entry["channel"] == channel))
        .collect();
    matching
        .iter()
        .find(|entry| entry["channel"] == "stable")
        .or(matching.first())
        .map(|entry| (*entry).clone())
}

/// Les notes à envoyer : celles du registre, complétées de ce que disent les drapeaux et les
/// `CHANGELOG*.md` de la version.
fn merged_notes(current: &Value, args: &NotesArgs, module_root: &std::path::Path) -> Result<Value> {
    let lang = changelog::default_lang(module_root);
    let extra = changelog::release_notes(
        &args.permission_reasons,
        args.host_action_required,
        &args.host_action,
        &lang,
    )?;
    let mut changelog: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for line in current["changelog"].as_array().into_iter().flatten() {
        for (lang, text) in line.as_object().into_iter().flatten() {
            changelog
                .entry(lang.clone())
                .or_default()
                .push(text.as_str().unwrap_or_default().to_string());
        }
    }
    // Une langue que `--notes` ou un fichier dit est remplacée entière ; les autres restent.
    let mut fresh: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for line in changelog::lines(&args.notes, &lang, module_root, &args.version)? {
        for (lang, text) in line {
            fresh.entry(lang).or_default().push(text);
        }
    }
    changelog.extend(fresh);
    let count = changelog.values().map(Vec::len).max().unwrap_or(0);
    let lines: Vec<BTreeMap<&str, &str>> = (0..count)
        .map(|index| {
            changelog
                .iter()
                .filter_map(|(lang, lines)| Some((lang.as_str(), lines.get(index)?.as_str())))
                .collect()
        })
        .collect();

    let mut reasons = current["permissionReasons"]
        .as_object()
        .cloned()
        .unwrap_or_default();
    for (permission, by_lang) in extra["permissionReasons"].as_object().into_iter().flatten() {
        let entry = reasons
            .entry(permission.clone())
            .or_insert_with(|| json!({}));
        for (lang, text) in by_lang.as_object().into_iter().flatten() {
            entry[lang] = text.clone();
        }
    }
    let mut action = current["hostAction"]
        .as_object()
        .cloned()
        .unwrap_or_default();
    for (lang, text) in extra["hostAction"].as_object().into_iter().flatten() {
        action.insert(lang.clone(), text.clone());
    }
    let required = current["hostActionRequired"].as_bool().unwrap_or(false)
        || extra["hostActionRequired"].as_bool().unwrap_or(false);
    Ok(json!({
        "changelog": lines,
        "permissionReasons": reasons,
        "hostActionRequired": required,
        "hostAction": action,
    }))
}

/// Ce qui manque, avec les mots de l'espace développeur.
fn print_missing(missing: &Value) {
    let items = missing.as_array().cloned().unwrap_or_default();
    if items.is_empty() {
        return;
    }
    ui::section("missing");
    for item in &items {
        ui::failure(missing_line(item));
    }
}

pub(crate) fn missing_line(item: &Value) -> String {
    let lang = item["lang"].as_str().unwrap_or("?");
    match item["kind"].as_str().unwrap_or_default() {
        "changelog" => format!("changelog ({lang})"),
        "changelogRewrite" => format!("changelog to rewrite ({lang}) — a line reads like a commit"),
        "hostAction" => format!("host action ({lang})"),
        "permissionReason" => format!(
            "reason for {} ({lang})",
            item["permission"].as_str().unwrap_or("?")
        ),
        "conformance" => format!(
            "check {} fails — fix the module and publish a new version",
            item["check"].as_str().unwrap_or("?")
        ),
        "conformance_pending" => {
            "checks still running — the version goes live by itself if they pass".to_string()
        }
        other => other.to_string(),
    }
}

fn signature_line(chain: &Value) -> String {
    match (
        chain["signature"].as_str(),
        chain["signatureSource"].as_str(),
    ) {
        (Some("signed"), Some(source)) => format!("signed ({source})"),
        (Some("signed"), None) => "signed".to_string(),
        (Some("unverified"), _) => "not verified — production will not run it".to_string(),
        _ => "unsigned — production will never run it".to_string(),
    }
}

/// La commande suivante pour cette version.
fn next_for(release: &Value, version: &str, modules: &ModuleArgs) -> (Option<String>, String) {
    let flag = modules
        .module
        .as_deref()
        .map(|id| format!(" --module {id}"))
        .unwrap_or_default();
    let kinds: Vec<&str> = release["missing"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| item["kind"].as_str())
        .collect();
    if kinds.contains(&"conformance") {
        return (
            Some(format!("portaki check{flag}")),
            "a blocking check fails on this digest — fix, then release a new version".to_string(),
        );
    }
    if kinds.contains(&"conformance_pending") {
        return (
            Some(format!("portaki release status {version}{flag}")),
            "checks are still running — look again in a moment".to_string(),
        );
    }
    if !kinds.is_empty() {
        return (
            Some(format!("portaki release notes {version} --complete{flag}")),
            "write what is missing (--notes, CHANGELOG.<lang>.md), then complete the version"
                .to_string(),
        );
    }
    if release["supplyChain"]["signature"] != "signed" {
        return (
            Some(format!("portaki release{flag}")),
            "sign the next version — production runs signed versions only".to_string(),
        );
    }
    (
        Some(format!("portaki reports --open{flag}")),
        "live — follow what hosts and the runtime report".to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stable_wins_when_a_version_is_on_both_channels() {
        let versions = json!([
            { "version": "1.0.0", "channel": "preview", "digest": "sha256:p" },
            { "version": "1.0.0", "channel": "stable", "digest": "sha256:s" },
            { "version": "0.9.0", "channel": "stable", "digest": "sha256:o" },
        ]);

        assert_eq!(
            pick(&versions, "1.0.0", None).unwrap()["digest"],
            "sha256:s"
        );
        assert_eq!(
            pick(&versions, "1.0.0", Some("preview")).unwrap()["digest"],
            "sha256:p"
        );
        assert!(pick(&versions, "2.0.0", None).is_none());
    }

    /// Une langue que `--notes` dit est remplacée ; celle qu'il ne dit pas reste celle du registre.
    #[test]
    fn completing_replaces_only_the_languages_given() {
        let dir = tempfile::tempdir().unwrap();
        let current = json!({
            "changelog": [{ "fr": "Ancienne", "en": "Old" }],
            "permissionReasons": { "email": { "fr": "Pour prévenir" } },
            "hostActionRequired": false,
            "hostAction": {},
        });
        let args = NotesArgs {
            version: "1.0.0".into(),
            channel: None,
            complete: true,
            notes: vec!["en:Faster sync".into()],
            permission_reasons: vec!["email=en:To warn".into()],
            host_action_required: false,
            host_action: vec![],
            modules: ModuleArgs::default(),
        };

        let body = merged_notes(&current, &args, dir.path()).unwrap();

        assert_eq!(
            body["changelog"],
            json!([{ "fr": "Ancienne", "en": "Faster sync" }])
        );
        assert_eq!(
            body["permissionReasons"]["email"],
            json!({ "fr": "Pour prévenir", "en": "To warn" })
        );
    }

    #[test]
    fn the_next_command_follows_what_is_missing() {
        let modules = ModuleArgs::default();
        let draft = json!({ "missing": [{ "kind": "changelog", "lang": "fr" }] });
        assert_eq!(
            next_for(&draft, "1.0.0", &modules).0.unwrap(),
            "portaki release notes 1.0.0 --complete"
        );
        let blocked = json!({ "missing": [{ "kind": "conformance", "check": "surfaces" }] });
        assert_eq!(
            next_for(&blocked, "1.0.0", &modules).0.unwrap(),
            "portaki check"
        );
        let live = json!({ "missing": [], "supplyChain": { "signature": "signed" } });
        assert_eq!(
            next_for(&live, "1.0.0", &modules).0.unwrap(),
            "portaki reports --open"
        );
    }
}
