//! `portaki status` — the developer space's home screen, in the terminal.
//!
//! The journey's five steps (`GET /dev/v1/onboarding`), then, for each module: the link to the
//! repository (`…/link`), the sandbox (`…/status`), the errors over 24 h and the open reports
//! (`/dev/v1/nav-counts`), the latest version (`/registry/v1/publications/mine`, then its
//! `…/release` page for what is missing and for the signature) — and the next command.
//!
//! No new route: these are the ones the developer space reads.

use anyhow::Result;
use clap::Parser;
use serde_json::{json, Value};

use crate::api::Platform;
use crate::workspace::ModuleArgs;
use crate::{auth, ui};

#[derive(Debug, Parser)]
/// Arguments for `portaki status`.
pub struct StatusArgs {
    #[command(flatten)]
    pub modules: ModuleArgs,
}

/// The five steps, in order and in the developer space's own words.
const JOURNEY: [(&str, &str, &str); 5] = [
    ("cliConnected", "Connect the CLI", "Connecter la CLI"),
    ("deployed", "Deploy to the sandbox", "Déployer en sandbox"),
    ("rendered", "Check the render", "Vérifier le rendu"),
    (
        "conformant",
        "Get conformance to green",
        "Passer la conformité au vert",
    ),
    ("published", "Publish", "Publier"),
];

/// What is known about a module, as `--json` renders it.
#[derive(Debug, Default, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ModuleStatus {
    id: String,
    /// `owner/repo` when the module is linked.
    repository: Option<String>,
    linked: bool,
    sandbox: Option<Value>,
    err24: u64,
    open_reports: u64,
    failed_checks: u64,
    latest: Option<Latest>,
    next: Next,
}

#[derive(Debug, Default, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Latest {
    version: String,
    digest: String,
    channel: String,
    /// `available` | `draft`
    state: String,
    missing: Vec<Value>,
    /// `signed` | `unsigned` | `unverified`
    signature: String,
    /// `ci` | `author`, absent when it is not signed.
    signature_source: Option<String>,
    /// The state of the review, as the registry names it.
    review: String,
    published_at: String,
}

/// The next command — or, when no command exists yet, what has to be done.
///
/// `reason` goes out in English in `--json`, whatever the language; `said` is what one reads.
#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize)]
struct Next {
    command: Option<String>,
    reason: String,
    #[serde(skip)]
    said: String,
}

fn next(command: Option<String>, reason: &str, french: &str) -> Next {
    Next {
        command,
        reason: reason.to_string(),
        said: if crate::lang::french() {
            french
        } else {
            reason
        }
        .to_string(),
    }
}

/// Runs `portaki status`.
pub async fn run(args: StatusArgs) -> Result<()> {
    ui::header(
        "portaki status",
        &crate::tr!(
            "Where the module stands on the developer journey — and what to run next.",
            "Où en est le module sur le parcours de l'espace développeur — et la commande suivante."
        ),
    );
    let members: Vec<String> = args
        .modules
        .resolve()?
        .into_iter()
        .map(|member| member.id)
        .filter(|id| !id.is_empty())
        .collect();
    let base = crate::profile::api_url(None);
    let flag = |id: &str| {
        if args.modules.all || args.modules.module.is_some() {
            format!(" --module {id}")
        } else {
            String::new()
        }
    };

    let mut platform = match Platform::open(&base) {
        Ok(platform) => platform,
        Err(failure) if failure.is::<auth::NotSignedIn>() => {
            let login = crate::profile::login_command(&base);
            return render(
                &base,
                None,
                &[],
                next(
                    Some(login),
                    "sign in — the journey is read from your account",
                    "connectez la CLI — le parcours se lit sur votre compte",
                ),
            );
        }
        Err(failure) => return Err(failure),
    };

    let journey = platform
        .get("/dev/v1/onboarding")
        .await?
        .unwrap_or(Value::Null);
    let counts = platform
        .get("/dev/v1/nav-counts")
        .await?
        .unwrap_or(Value::Null);
    let mine = platform
        .get("/registry/v1/publications/mine?page=0&size=100")
        .await?
        .unwrap_or(Value::Null);

    let mut modules = Vec::new();
    for id in &members {
        let mut module = ModuleStatus {
            id: id.clone(),
            ..Default::default()
        };
        let per = &counts["perModule"][id];
        module.err24 = per["err24"].as_u64().unwrap_or(0);
        module.open_reports = per["openReports"].as_u64().unwrap_or(0);
        module.failed_checks = per["failedChecks"].as_u64().unwrap_or(0);
        let link = platform.get(&format!("/dev/v1/modules/{id}/link")).await?;
        module.linked = link.is_some();
        module.repository = link
            .as_ref()
            .and_then(|link| link["repository"].as_str())
            .map(str::to_string);
        module.sandbox = platform
            .get(&format!("/dev/v1/modules/{id}/status"))
            .await?;
        module.latest = match latest_of(&mine, id) {
            Some(mut latest) => {
                let release = platform
                    .get(&format!(
                        "/registry/v1/publications/{}/release",
                        latest.digest
                    ))
                    .await?
                    .unwrap_or(Value::Null);
                read_release(&mut latest, &release);
                Some(latest)
            }
            None => None,
        };
        module.next = decide(&journey, &module, &flag(id));
        modules.push(module);
    }

    let first = modules
        .first()
        .map(|module| module.next.clone())
        .unwrap_or_else(|| {
            next(
                Some("portaki init <name>".to_string()),
                "no module here — create one, or pass --module <id> from its repository",
                "aucun module ici — créez-en un, ou passez --module <id> depuis son dépôt",
            )
        });
    render(&base, Some(&journey), &modules, first)
}

/// The most recent publication of this module: `mine` returns them from the most recent to the
/// oldest.
fn latest_of(mine: &Value, id: &str) -> Option<Latest> {
    let item = mine["items"]
        .as_array()?
        .iter()
        .find(|item| item["moduleId"] == id)?;
    let text = |key: &str| item[key].as_str().unwrap_or_default().to_string();
    Some(Latest {
        version: text("version"),
        digest: text("digest"),
        channel: text("channel"),
        state: text("releaseState"),
        review: text("status"),
        published_at: text("publishedAt"),
        signature: "unsigned".to_string(),
        ..Default::default()
    })
}

/// The release page: what a draft is missing, and who signed it.
fn read_release(latest: &mut Latest, release: &Value) {
    if let Some(state) = release["releaseState"].as_str() {
        latest.state = state.to_string();
    }
    latest.missing = release["missing"].as_array().cloned().unwrap_or_default();
    let chain = &release["supplyChain"];
    if let Some(signature) = chain["signature"].as_str() {
        latest.signature = signature.to_string();
    }
    latest.signature_source = chain["signatureSource"].as_str().map(str::to_string);
}

/// The next command, from the most blocking to the most comfortable.
fn decide(journey: &Value, module: &ModuleStatus, flag: &str) -> Next {
    let step = |key: &str| journey[key].as_bool().unwrap_or(false);
    let deployed = module
        .sandbox
        .as_ref()
        .is_some_and(|sandbox| !sandbox["lastDeploy"].is_null());
    if !deployed {
        return next(
            Some(format!("portaki dev --watch{flag}")),
            "deploy to the sandbox — it rebuilds and redeploys on every save",
            "déployer en sandbox — recompile et redéploie à chaque sauvegarde",
        );
    }
    if !step("rendered") {
        return next(
            Some(format!("portaki preview{flag}")),
            "check the render in the developer space sandbox, where hosts and guests will see it",
            "vérifier le rendu, là où l'hôte et le voyageur le verront",
        );
    }
    if module.failed_checks > 0 || !step("conformant") {
        return next(
            Some(format!("portaki check{flag}")),
            "get conformance to green before publishing",
            "passer la conformité au vert avant de publier",
        );
    }
    if !module.linked {
        return next(
            Some(format!("portaki link{flag}")),
            "link the module to its repository — publishing from CI needs it",
            "lier le module à son dépôt — publier depuis la CI l'exige",
        );
    }
    let Some(latest) = &module.latest else {
        return next(
            Some(format!("portaki release{flag}")),
            "publish a first version",
            "publier une première version",
        );
    };
    if latest.state == "draft" {
        return next(
            Some(format!("portaki release notes {}{flag}", latest.version)),
            "complete the release notes of the draft — it stays invisible to hosts until then",
            "compléter la version en brouillon — invisible des hôtes jusque-là",
        );
    }
    if module.err24 > 0 {
        return next(
            Some(format!("portaki logs{flag}")),
            "errors in the last 24 hours — follow the sandbox logs",
            "des erreurs sur 24 h — suivre les journaux de la sandbox",
        );
    }
    if module.open_reports > 0 {
        return next(
            Some(format!("portaki reports --open{flag}")),
            "open reports are waiting — fix, then resolve them",
            "des rapports ouverts attendent — corrigez, puis marquez-les résolus",
        );
    }
    if latest.signature != "signed" {
        return next(
            Some(format!("portaki release{flag}")),
            "sign the next version — production runs signed versions only",
            "signer la prochaine version — la production n'exécute que des versions signées",
        );
    }
    next(
        Some(format!("portaki dev --watch{flag}")),
        "all set — keep iterating in the sandbox",
        "tout est en ordre — continuez en sandbox",
    )
}

fn render(
    base: &str,
    journey: Option<&Value>,
    modules: &[ModuleStatus],
    first: Next,
) -> Result<()> {
    if ui::json() {
        let steps: serde_json::Map<String, Value> = JOURNEY
            .iter()
            .map(|(key, _, _)| {
                let done = journey.and_then(|journey| journey[*key].as_bool());
                ((*key).to_string(), json!(done.unwrap_or(false)))
            })
            .collect();
        ui::emit(&json!({
            "schemaVersion": 1,
            "api": base,
            "signedIn": journey.is_some(),
            "journey": steps,
            "modules": modules,
            "next": first,
        }));
        return Ok(());
    }

    ui::field("api", base);
    ui::section(&crate::tr!("journey", "parcours"));
    for (key, en, fr) in JOURNEY {
        let title = if crate::lang::french() { fr } else { en };
        match journey.and_then(|journey| journey[key].as_bool()) {
            Some(true) => ui::success(title),
            _ => ui::skipped(title),
        }
    }
    for module in modules {
        ui::section(&module.id);
        ui::field(
            "repository",
            module.repository.as_deref().unwrap_or(if module.linked {
                "linked"
            } else {
                "not linked"
            }),
        );
        ui::field("sandbox", sandbox_line(module.sandbox.as_ref()));
        match &module.latest {
            Some(latest) => {
                ui::field("latest", latest_line(latest));
                for missing in &latest.missing {
                    ui::detail(format!("  missing: {}", missing_line(missing)));
                }
            }
            None => ui::field("latest", "never published"),
        }
        ui::field("errors 24h", module.err24);
        ui::field("reports", format!("{} open", module.open_reports));
        if module.failed_checks > 0 {
            ui::field("checks", format!("{} failing", module.failed_checks));
        }
        if modules.len() > 1 {
            ui::field("next", next_line(&module.next));
        }
    }
    match &first.command {
        Some(command) => ui::next(&[(command, &first.said)]),
        None => {
            ui::section(&crate::tr!("next", "ensuite"));
            ui::detail(&first.said);
        }
    }
    ui::blank();
    Ok(())
}

fn sandbox_line(sandbox: Option<&Value>) -> String {
    let Some(sandbox) = sandbox else {
        return "nothing deployed".to_string();
    };
    let deploy = &sandbox["lastDeploy"];
    if deploy.is_null() {
        return "nothing deployed".to_string();
    }
    let mut line = format!(
        "{} deployed {}",
        deploy["version"].as_str().unwrap_or("?"),
        deploy["at"].as_str().unwrap_or("")
    );
    if let Some(run) = sandbox["lastRun"]["status"].as_str() {
        line.push_str(&format!(" · last run {run}"));
    }
    if sandbox["watchConnected"].as_bool() == Some(true) {
        line.push_str(" · watch connected");
    }
    line
}

fn latest_line(latest: &Latest) -> String {
    let signature = match &latest.signature_source {
        Some(source) => format!("{} ({source})", latest.signature),
        None => latest.signature.clone(),
    };
    format!(
        "{} · {} · {} · {} · review {}",
        latest.version, latest.channel, latest.state, signature, latest.review
    )
}

fn missing_line(missing: &Value) -> String {
    let field = |key: &str| missing[key].as_str().unwrap_or("?");
    match missing["permission"].as_str() {
        Some(permission) => format!("{} {permission} ({})", field("kind"), field("lang")),
        None => format!("{} ({})", field("kind"), field("lang")),
    }
}

fn next_line(next: &Next) -> String {
    match &next.command {
        Some(command) => format!("{command} — {}", next.said),
        None => next.said.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deployed() -> Option<Value> {
        Some(json!({ "lastDeploy": { "version": "1.0.0", "at": "2026-09-27T10:00:00Z" } }))
    }

    fn all_steps() -> Value {
        json!({ "cliConnected": true, "deployed": true, "rendered": true, "conformant": true, "published": true })
    }

    fn published(state: &str, signature: &str) -> Option<Latest> {
        Some(Latest {
            state: state.into(),
            signature: signature.into(),
            ..Default::default()
        })
    }

    /// The journey in the developer space's own order: each missing step gives its command, and
    /// the first missing one wins.
    #[test]
    fn the_next_command_follows_the_journey() {
        let mut module = ModuleStatus::default();
        assert_eq!(
            decide(&all_steps(), &module, "").command.as_deref(),
            Some("portaki dev --watch")
        );

        module.sandbox = deployed();
        let mut steps = all_steps();
        steps["conformant"] = json!(false);
        assert_eq!(
            decide(&steps, &module, " --module nuki").command.as_deref(),
            Some("portaki check --module nuki")
        );

        assert_eq!(
            decide(&all_steps(), &module, "").command.as_deref(),
            Some("portaki link")
        );

        module.linked = true;
        assert_eq!(
            decide(&all_steps(), &module, "").command.as_deref(),
            Some("portaki release")
        );

        module.latest = published("draft", "signed");
        let draft = decide(&all_steps(), &module, "");
        assert_eq!(
            draft.command.as_deref(),
            Some(
                format!(
                    "portaki release notes {}",
                    module.latest.as_ref().unwrap().version
                )
                .as_str()
            )
        );
        assert!(draft.reason.contains("release notes"));

        module.latest = published("available", "unsigned");
        assert_eq!(
            decide(&all_steps(), &module, "").command.as_deref(),
            Some("portaki release")
        );

        module.err24 = 3;
        assert_eq!(
            decide(&all_steps(), &module, "").command.as_deref(),
            Some("portaki logs")
        );
    }

    /// `mine` is sorted from the most recent to the oldest: the module's first row is the one.
    #[test]
    fn the_latest_version_is_the_first_of_the_module_and_the_release_completes_it() {
        let mine = json!({ "items": [
            { "moduleId": "wifi", "version": "3.0.0", "digest": "sha256:w" },
            { "moduleId": "nuki", "version": "1.2.0", "digest": "sha256:b", "channel": "stable",
              "releaseState": "available", "status": "pending" },
            { "moduleId": "nuki", "version": "1.1.0", "digest": "sha256:a" },
        ]});

        let mut latest = latest_of(&mine, "nuki").unwrap();
        assert_eq!(latest.version, "1.2.0");
        assert_eq!(latest.signature, "unsigned");

        read_release(
            &mut latest,
            &json!({
                "releaseState": "draft",
                "missing": [{ "kind": "changelog", "lang": "en" }],
                "supplyChain": { "signature": "signed", "signatureSource": "author" },
            }),
        );
        assert_eq!(latest.state, "draft");
        assert_eq!(latest.signature_source.as_deref(), Some("author"));
        assert_eq!(missing_line(&latest.missing[0]), "changelog (en)");
        assert!(latest_of(&mine, "absent").is_none());
    }
}
