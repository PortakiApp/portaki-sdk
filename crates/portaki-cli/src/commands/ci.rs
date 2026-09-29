//! `portaki ci` — what a workflow has had to do in bash until now.
//!
//! A module repository used to drive its CI with two hundred lines of `bash`, `jq` and `curl`:
//! discovering the changed modules, resolving which CLI to install, announcing a run. All three
//! are questions about a module — the CLI knows more about them than a script does, and it is
//! already installed on the runner.
//!
//! Every subcommand writes to standard output **and** to `GITHUB_OUTPUT` when it exists: readable
//! by hand, consumable by a following step, with no second form to keep up to date.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use portaki_sdk::manifest::ModuleManifest;

use crate::manifest::generator::{MANIFEST_VERSION, SDUI_SCHEMA_VERSION};
use crate::ui;
use crate::update::outdated;

/// The manifest that makes a directory a module.
use crate::manifest::source::{self, MODULE_MANIFEST};

/// The directory where a multi-module repository keeps them.
const MODULES_DIR: &str = "modules";

/// What, once changed, forces a full rebuild: the foundation shared by every module.
const SHARED_PATHS: [&str; 5] = [
    "Cargo.toml",
    "Cargo.lock",
    ".cargo/",
    "rust-toolchain",
    "rust-toolchain.toml",
];

#[derive(Debug, Parser)]
/// Arguments for `portaki ci`.
pub struct CiArgs {
    #[command(subcommand)]
    pub command: CiCommand,
}

#[derive(Debug, Subcommand)]
/// The questions a workflow asks about a module repository.
pub enum CiCommand {
    /// List the modules to build — all of them, or only those a change touched.
    Modules(ModulesArgs),
    /// Print the Portaki SDK this checkout actually resolves to, as a cache key.
    SdkVersion(SdkVersionArgs),
    /// Warn about what will age badly: an old SDK, a manifest behind the host.
    Check(CheckArgs),
    /// Print this module's id and version — what a workflow needs to name a release.
    Info(InfoArgs),
    /// Tell Portaki how this run ended, so a broken module raises an alert — and a fixed one
    /// clears it.
    Report(ReportArgs),
    /// Test, build and package, pushing nothing — the job that runs the module's code, without
    /// publishing rights. Hand target/portaki and the wasm to the job that runs ci release.
    Build(crate::commands::release::ReleaseArgs),
    /// Push what ci build produced to Portaki's registry, sign it as this workflow (provenance,
    /// cargo audit), and announce it. Runs nothing of the module, not even cargo.
    Release(CiReleaseArgs),
}

#[derive(Debug, Parser)]
/// Arguments for `portaki ci release`.
pub struct CiReleaseArgs {
    #[command(flatten)]
    pub release: crate::commands::release::ReleaseArgs,
    /// The `cargo audit` report to attest next to the provenance — produced in this job, from
    /// Cargo.lock, never taken from the build job. Without it, the version shows as not audited.
    #[arg(long, value_name = "FILE")]
    pub audit: Option<PathBuf>,
}

/// `--root` (hidden, legacy) or `--module` / `--all`: the roots of the modules being aimed at.
fn roots(root: &Option<PathBuf>, modules: &crate::workspace::ModuleArgs) -> Result<Vec<PathBuf>> {
    if let Some(root) = root {
        return Ok(vec![root.clone()]);
    }
    Ok(modules
        .resolve()?
        .into_iter()
        .map(|member| member.root)
        .collect())
}

/// Runs `portaki ci`.
pub async fn run(args: CiArgs) -> Result<()> {
    match args.command {
        CiCommand::Modules(args) => modules(args),
        CiCommand::SdkVersion(args) => sdk_version(args),
        CiCommand::Check(args) => check(args).await,
        CiCommand::Info(args) => info(args),
        CiCommand::Report(args) => report(args).await,
        CiCommand::Build(args) => {
            crate::commands::release::run_as(args, crate::commands::release::Mode::CiBuild).await
        }
        CiCommand::Release(args) => {
            let mode = crate::commands::release::Mode::CiRelease { audit: args.audit };
            crate::commands::release::run_as(args.release, mode).await
        }
    }
}

#[derive(Debug, Parser)]
/// Arguments for `portaki ci report`.
pub struct ReportArgs {
    /// How the run ended — `success`, `failure`, `cancelled`.
    #[arg(long)]
    pub outcome: String,
    /// Where to look at it. Defaults to the current GitHub Actions run.
    #[arg(long)]
    pub run_url: Option<String>,
    /// Former form of --module: the module root.
    #[arg(long, hide = true)]
    pub root: Option<PathBuf>,
    #[command(flatten)]
    pub modules: crate::workspace::ModuleArgs,
    /// Alias of the global --api, kept for older scripts.
    #[arg(long, hide = true)]
    pub url: Option<String>,
}

/// Tells Portaki how this run ended.
///
/// Nothing is stored on the module side: Portaki has no right to read your runs. It is the OIDC
/// token — issued by GitHub for the lifetime of the job — that proves where the report comes from.
///
/// To be called on *every* outcome, not only failures: made conditional on failure, this report
/// could never clear an alert, and a module that had been repaired would keep its own for ever.
///
/// A report that fails does not fail the run. It comes after the publication, which has already
/// happened; turning a job red over a status report would invert the importance of the two.
async fn report(args: ReportArgs) -> Result<()> {
    let base = crate::profile::api_url(args.url.as_deref());
    let run_url = args.run_url.clone().or_else(github_run_url);
    let mut reported = Vec::new();
    for root in roots(&args.root, &args.modules)? {
        let module_id = read_module_id(&root)?;
        let delivered = deliver(&base, &module_id, &args.outcome, run_url.as_deref()).await;
        match &delivered {
            Ok(()) => ui::success(format!("reported {} for {module_id}", args.outcome)),
            Err(failure) => {
                ui::skipped(format!("could not report the run: {failure}"));
                ui::advice("the publication itself is unaffected — only the alert was not updated");
            }
        }
        reported.push(serde_json::json!({
            "id": module_id,
            "outcome": args.outcome,
            "reported": delivered.is_ok(),
            "error": delivered.err().map(|failure| failure.to_string()),
        }));
    }
    if ui::json() {
        ui::emit(&serde_json::json!({ "schemaVersion": 1, "modules": reported }));
    }
    if !ui::plain() {
        ui::blank();
    }
    Ok(())
}

async fn deliver(base: &str, module_id: &str, outcome: &str, run_url: Option<&str>) -> Result<()> {
    crate::auth::ensure_transport(base)?;
    if !crate::oidc::available() {
        anyhow::bail!("no OIDC token available — add `permissions: id-token: write` to the job");
    }
    let token = crate::oidc::request_token(&crate::oidc::audience(base)).await?;
    let response = crate::http::client()
        .post(format!("{}/registry/v1/runs", base.trim_end_matches('/')))
        .bearer_auth(token)
        .json(&serde_json::json!({
            "moduleId": module_id,
            "outcome": outcome,
            "runUrl": run_url.unwrap_or_default(),
        }))
        .send()
        .await?;
    let status = response.status();
    if !status.is_success() {
        anyhow::bail!("the registry answered {status}");
    }
    Ok(())
}

/// This run's address, rebuilt from what GitHub Actions puts in the environment.
fn github_run_url() -> Option<String> {
    let server = std::env::var("GITHUB_SERVER_URL").ok()?;
    let repository = std::env::var("GITHUB_REPOSITORY").ok()?;
    let run = std::env::var("GITHUB_RUN_ID").ok()?;
    Some(format!("{server}/{repository}/actions/runs/{run}"))
}

/// The module's identifier, read where it is declared.
///
/// The template used to take it from the repository name — wrong as soon as one repository holds
/// several, and fragile even with a single one: nothing obliges a repository to carry the name of
/// its module.
fn read_module_id(root: &Path) -> Result<String> {
    source::module_id(root).with_context(|| {
        format!(
            "{} is not a module — run from the module root",
            root.display()
        )
    })
}

#[derive(Debug, Parser)]
/// Arguments for `portaki ci info`.
pub struct InfoArgs {
    /// Former form of --module: the module root.
    #[arg(long, hide = true)]
    pub root: Option<PathBuf>,
    #[command(flatten)]
    pub modules: crate::workspace::ModuleArgs,
}

/// The module's identity, for a workflow that has to name it.
///
/// Without it, a composite action was reduced to extracting the version from the manifest with
/// `python3` or `jq` — one more dependency on the runner, for a field the CLI already reads.
fn info(args: InfoArgs) -> Result<()> {
    let mut found = Vec::new();
    for root in roots(&args.root, &args.modules)? {
        let (id, version) = source::coordinates(&root)
            .filter(|(id, version)| !id.is_empty() && !version.is_empty())
            .with_context(|| {
                format!(
                    "{} carries no module id or no version — run from the module root",
                    root.display()
                )
            })?;
        found.push((id, version));
    }
    if ui::json() {
        let modules: Vec<serde_json::Value> = found
            .iter()
            .map(|(id, version)| serde_json::json!({ "id": id, "version": version }))
            .collect();
        ui::emit(&serde_json::json!({ "schemaVersion": 1, "modules": modules }));
        return Ok(());
    }
    for (id, version) in found {
        info_one(&id, &version)?;
    }
    Ok(())
}

fn info_one(id: &str, version: &str) -> Result<()> {
    emit_outputs(&[("id", id), ("version", version)])?;

    if ui::plain() {
        // Two lines, in a fixed order: `read id version < <(portaki --plain ci info)`.
        println!("{id}");
        println!("{version}");
        return Ok(());
    }
    ui::header("portaki ci info", "What this module calls itself.");
    ui::field("id", id);
    ui::field("version", version);
    ui::blank();
    Ok(())
}

#[derive(Debug, Parser)]
/// Arguments for `portaki ci modules`.
pub struct ModulesArgs {
    /// Repository root (defaults to the current directory).
    #[arg(long, hide = true)]
    pub root: Option<PathBuf>,
    #[command(flatten)]
    pub modules: crate::workspace::ModuleArgs,
    /// Keep only the modules changed since this git ref.
    #[arg(long)]
    pub changed_since: Option<String>,
    /// Restrict to these modules, whatever changed.
    #[arg(long, value_delimiter = ',')]
    pub only: Vec<String>,
}

/// What was found, and why.
///
/// The reason is not decoration: "nothing changed" and "I could not compare" produce the same
/// empty list and do not call for the same reaction.
struct Selection {
    modules: Vec<String>,
    reason: &'static str,
}

fn modules(args: ModulesArgs) -> Result<()> {
    let root = args
        .root
        .clone()
        .map(Ok)
        .unwrap_or_else(std::env::current_dir)
        .context("resolve the repository root")?;

    let known = discover(&root)?;
    if known.is_empty() {
        anyhow::bail!(
            "no module found — expected a crate on portaki-sdk here, or one under {MODULES_DIR}/*/"
        );
    }

    let mut args = args;
    if let Some(module) = args.modules.module.clone() {
        args.only.push(module);
    }
    let selection = if args.modules.all {
        Selection {
            modules: known.clone(),
            reason: "asked for",
        }
    } else {
        select(&root, &known, &args)?
    };
    let json = serde_json::to_string(&selection.modules)?;

    // The workflow reads `modules` for its matrix and `any` to skip the jobs: without `any`, an
    // empty matrix fails the job instead of skipping it.
    emit_outputs(&[
        ("modules", &json),
        ("any", &(!selection.modules.is_empty()).to_string()),
        ("reason", selection.reason),
    ])?;

    if ui::json() {
        ui::emit(&serde_json::json!({
            "schemaVersion": 1,
            "modules": selection.modules,
            "any": !selection.modules.is_empty(),
            "reason": selection.reason,
        }));
        return Ok(());
    }
    if ui::plain() {
        println!("{json}");
        return Ok(());
    }

    ui::header(
        "portaki ci modules",
        "The modules this run should build, and why that is the list.",
    );
    if selection.modules.is_empty() {
        ui::skipped(format!("nothing to build ({})", selection.reason));
    } else {
        ui::detail(selection.reason);
        // The column gives the declared version rather than the reason: the reason holds for the
        // whole list, repeating it twenty times teaches nothing, and the version is what the eye
        // goes looking for when re-reading a release.
        let versions: Vec<String> = selection
            .modules
            .iter()
            .map(|name| declared_version(&root, name).unwrap_or_else(|| "?".to_string()))
            .collect();
        let rows: Vec<(&str, &str)> = selection
            .modules
            .iter()
            .zip(&versions)
            .map(|(name, version)| (name.as_str(), version.as_str()))
            .collect();
        ui::list("modules", &rows);
    }
    ui::blank();
    Ok(())
}

/// The version a module declares, to show it opposite its name.
fn declared_version(root: &Path, name: &str) -> Option<String> {
    [root.join(MODULES_DIR).join(name), root.to_path_buf()]
        .iter()
        .find(|candidate| source::is_module(candidate))
        .and_then(|candidate| source::coordinates(candidate))
        .map(|(_, version)| version)
}

/// The repository's modules: `modules/*` if there are any, otherwise the current directory itself.
///
/// The two layouts coexist — one repository per module, or one repository gathering them — and
/// neither is declared anywhere. A crate on `portaki-sdk` — or a manifest — is what settles it.
fn discover(root: &Path) -> Result<Vec<String>> {
    let nested = root.join(MODULES_DIR);
    if nested.is_dir() {
        let mut found = Vec::new();
        for entry in std::fs::read_dir(&nested)
            .with_context(|| format!("read {}", nested.display()))?
            .flatten()
        {
            if source::is_module(&entry.path()) {
                found.push(entry.file_name().to_string_lossy().to_string());
            }
        }
        found.sort();
        if !found.is_empty() {
            return Ok(found);
        }
    }
    if source::is_module(root) {
        let name = root
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| ".".to_string());
        return Ok(vec![name]);
    }
    Ok(Vec::new())
}

fn select(root: &Path, known: &[String], args: &ModulesArgs) -> Result<Selection> {
    if !args.only.is_empty() {
        let mut chosen = Vec::new();
        for name in &args.only {
            let name = name.trim();
            if name.is_empty() {
                continue;
            }
            if !known.iter().any(|candidate| candidate == name) {
                anyhow::bail!("unknown module: {name}");
            }
            chosen.push(name.to_string());
        }
        return Ok(Selection {
            modules: chosen,
            reason: "asked for",
        });
    }

    let Some(base) = args.changed_since.as_deref() else {
        return Ok(Selection {
            modules: known.to_vec(),
            reason: "no base to compare against",
        });
    };

    // A base that is missing or all zeros means "first push", "force-push" or "shallow clone".
    // Rebuilding everything costs minutes there; rebuilding nothing would lose a publication,
    // which costs more.
    if base.trim().is_empty() || base.chars().all(|character| character == '0') {
        return Ok(Selection {
            modules: known.to_vec(),
            reason: "no usable base",
        });
    }

    let Some(changed) = changed_paths(root, base) else {
        return Ok(Selection {
            modules: known.to_vec(),
            reason: "base not found",
        });
    };

    if changed.is_empty() {
        return Ok(Selection {
            modules: Vec::new(),
            reason: "nothing changed",
        });
    }

    if changed.iter().any(|path| touches_shared(path)) {
        return Ok(Selection {
            modules: known.to_vec(),
            reason: "the shared workspace changed",
        });
    }

    let mut touched: Vec<String> = known
        .iter()
        .filter(|name| {
            let prefix = format!("{MODULES_DIR}/{name}/");
            changed.iter().any(|path| path.starts_with(&prefix))
        })
        .cloned()
        .collect();
    touched.sort();
    touched.dedup();

    if touched.is_empty() {
        return Ok(Selection {
            modules: Vec::new(),
            reason: "no module was touched",
        });
    }
    Ok(Selection {
        modules: touched,
        reason: "changed",
    })
}

/// The paths changed since `base`, or `None` if git cannot make the comparison.
fn changed_paths(root: &Path, base: &str) -> Option<Vec<String>> {
    let output = std::process::Command::new("git")
        .current_dir(root)
        .args(["diff", "--name-only", &format!("{base}...HEAD")])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::to_string)
            .filter(|path| !path.is_empty())
            .collect(),
    )
}

/// Does this path belong to the shared foundation?
///
/// Matched on prefixes, not on a pattern: `.github/workflows/ci.yml` must not set off the whole
/// matrix, and a slightly too broad pattern would do exactly that without anyone seeing it.
fn touches_shared(path: &str) -> bool {
    SHARED_PATHS.iter().any(|shared| {
        if let Some(directory) = shared.strip_suffix('/') {
            path.starts_with(&format!("{directory}/"))
        } else {
            path == *shared
        }
    })
}

#[derive(Debug, Parser)]
/// Arguments for `portaki ci sdk-version`.
pub struct SdkVersionArgs {
    /// Former form of --module: the directory holding the `Cargo.lock` to read.
    #[arg(long, hide = true)]
    pub root: Option<PathBuf>,
    #[command(flatten)]
    pub modules: crate::workspace::ModuleArgs,
}

/// The SDK this checkout resolves to, and the CLI version to install alongside it.
///
/// Read from `Cargo.lock`, not from `Cargo.toml`: a module may declare the SDK by semver, by git
/// branch or by path, and only the lock says what will actually be compiled.
///
/// The key returned is the version alone, because the CLI is installed from crates.io —
/// `cargo install portaki-cli@<version>`. Cloning the SDK repository to compile it there cost a
/// branch resolution on every run, a cache invalidated on every SDK commit, and a binary that was
/// no published version's. The revision is still returned for information: it says the checkout
/// follows a branch, not a release.
fn sdk_version(args: SdkVersionArgs) -> Result<()> {
    // A monorepo shares its lock: the first module is enough.
    let root = roots(&args.root, &args.modules)?
        .into_iter()
        .next()
        .context("resolve the checkout root")?;
    let lock = find_lockfile(&root).context(
        "no Cargo.lock found here or above — the SDK a build resolves to is written there",
    )?;
    let text =
        std::fs::read_to_string(&lock).with_context(|| format!("read {}", lock.display()))?;
    let resolved = read_locked_sdk(&text).context("Cargo.lock carries no portaki-sdk")?;

    emit_outputs(&[
        ("version", &resolved.version),
        ("rev", resolved.rev.as_deref().unwrap_or("")),
        ("key", resolved.cache_key()),
    ])?;

    if ui::json() {
        ui::emit(&serde_json::json!({
            "schemaVersion": 1,
            "version": resolved.version,
            "rev": resolved.rev,
            "key": resolved.cache_key(),
        }));
        return Ok(());
    }
    if ui::plain() {
        println!("{}", resolved.cache_key());
        return Ok(());
    }

    ui::header(
        "portaki ci sdk-version",
        "The Portaki SDK this checkout resolves to — the key a CLI cache should use.",
    );
    ui::field("version", &resolved.version);
    if let Some(rev) = &resolved.rev {
        ui::field("rev", rev);
    }
    ui::field("install", format!("portaki-cli@{}", resolved.cache_key()));
    if resolved.rev.is_some() {
        ui::advice("this checkout follows a git branch — the published CLI may lag behind it");
    }
    ui::blank();
    Ok(())
}

/// The SDK the lock resolves to.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct LockedSdk {
    pub(crate) version: String,
    /// The exact revision, when the SDK comes from a git repository rather than from crates.io.
    rev: Option<String>,
}

impl LockedSdk {
    /// The version to install, and the cache key that goes with it — the two are the same thing.
    fn cache_key(&self) -> &str {
        &self.version
    }
}

/// The nearest `Cargo.lock`, walking upwards: a module of a multi-module repository shares the
/// one at the root.
pub(crate) fn find_lockfile(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .map(|directory| directory.join("Cargo.lock"))
        .find(|candidate| candidate.is_file())
}

/// Extracts `portaki-sdk` from the lock, with no extra dependency.
///
/// The format is stable and trivial — `[[package]]` blocks of `key = "value"` lines. Adding a
/// full TOML parser to the CLI for two fields would cost more than it protects.
pub(crate) fn read_locked_sdk(lock: &str) -> Option<LockedSdk> {
    let mut in_sdk = false;
    let mut version = None;
    let mut source = None;

    for line in lock.lines() {
        let line = line.trim();
        if line == "[[package]]" {
            if in_sdk {
                break;
            }
            version = None;
            source = None;
            continue;
        }
        if let Some(value) = quoted(line, "name") {
            in_sdk = value == "portaki-sdk";
            continue;
        }
        if let Some(value) = quoted(line, "version") {
            version = Some(value.to_string());
        }
        if let Some(value) = quoted(line, "source") {
            source = Some(value.to_string());
        }
        if in_sdk && version.is_some() && line.starts_with("dependencies") {
            break;
        }
    }

    let version = version?;
    if !in_sdk {
        return None;
    }
    let rev = source
        .and_then(|source| source.split_once('#').map(|(_, rev)| rev.to_string()))
        .filter(|rev| !rev.is_empty());
    Some(LockedSdk { version, rev })
}

/// `key = "value"` → `value`.
fn quoted<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let rest = line.strip_prefix(key)?.trim_start();
    let rest = rest.strip_prefix('=')?.trim();
    rest.strip_prefix('"')?.strip_suffix('"')
}

#[derive(Debug, Parser)]
/// Arguments for `portaki ci check`.
pub struct CheckArgs {
    /// Former form of --module: the module root.
    #[arg(long, hide = true)]
    pub root: Option<PathBuf>,
    #[command(flatten)]
    pub modules: crate::workspace::ModuleArgs,
    /// Skip the crates.io lookup — for an offline runner.
    #[arg(long)]
    pub offline: bool,
}

/// What gets in nobody's way today and will cost dearly later.
async fn check(args: CheckArgs) -> Result<()> {
    if !ui::plain() {
        ui::header(
            "portaki ci check",
            "What still builds today and will not tomorrow.",
        );
    }
    let mut modules = Vec::new();
    for root in roots(&args.root, &args.modules)? {
        let (id, warnings) = check_one(&root, args.offline).await?;
        modules.push(serde_json::json!({ "id": id, "warnings": warnings }));
    }
    if ui::json() {
        ui::emit(&serde_json::json!({ "schemaVersion": 1, "modules": modules }));
    }
    if !ui::plain() {
        ui::blank();
    }
    Ok(())
}

/// `ci check` for one module: what is ageing, said as it goes and returned for `--json`.
async fn check_one(root: &Path, offline: bool) -> Result<(String, Vec<serde_json::Value>)> {
    let root = root.to_path_buf();
    warnings().clear();
    let mut found = 0;
    let (manifest, _) = crate::manifest::load_manifest(&root, None)?;

    if manifest.manifest_version != MANIFEST_VERSION {
        found += 1;
        annotate(
            Some(MODULE_MANIFEST),
            format!(
                "manifest_version {} — this SDK writes {MANIFEST_VERSION}; rebuild with a current \
                 portaki build",
                manifest.manifest_version
            ),
        );
    }
    for (shell, declared) in [
        ("host", &manifest.ui_schema.host),
        ("guest", &manifest.ui_schema.guest),
    ] {
        if declared != SDUI_SCHEMA_VERSION {
            found += 1;
            annotate(
                Some(MODULE_MANIFEST),
                format!(
                    "ui_schema {shell}={declared} — the shell now renders {SDUI_SCHEMA_VERSION}; \
                     newer primitives will not be available"
                ),
            );
        }
    }

    if offline {
        ui::skipped("skipped the crates.io lookup (--offline)");
    } else if let Some(lock) = find_lockfile(&root) {
        let text = std::fs::read_to_string(&lock)?;
        if let Some(resolved) = read_locked_sdk(&text) {
            match crate::update::latest_published("portaki-sdk").await {
                Ok(latest) if outdated(&resolved.version, &latest) => {
                    found += 1;
                    annotate(
                        Some("Cargo.toml"),
                        format!(
                            "portaki-sdk {} — {latest} is published; \
                             https://github.com/PortakiApp/portaki-sdk/releases",
                            resolved.version
                        ),
                    );
                }
                Ok(_) => {}
                // An unreachable registry is not a defect of the module: say so, and carry on.
                Err(failure) => ui::skipped(format!("could not reach crates.io: {failure}")),
            }

            match withdrawn(&resolved.version).await {
                Ok(catalogue) => found += warn_about_withdrawn(&manifest, &catalogue),
                // The registry is the source, but its being unavailable is not a defect of the
                // module: a build must not fail because a platform is answering badly.
                Err(failure) => ui::skipped(format!("could not read the deprecations: {failure}")),
            }
        }
    }

    if found == 0 {
        ui::success(format!("{} has nothing ageing", manifest.id));
    }
    let said = warnings().drain(..).collect();
    Ok((manifest.id.clone(), said))
}

/// What `annotate` said during the `check_one` currently running, for `--json`. A lock and not a
/// `thread_local`: the task moves from thread to thread at every `await`.
static WARNINGS: std::sync::Mutex<Vec<serde_json::Value>> = std::sync::Mutex::new(Vec::new());

fn warnings() -> std::sync::MutexGuard<'static, Vec<serde_json::Value>> {
    WARNINGS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Writes the warning where it will be seen.
///
/// Under GitHub Actions, the `::warning::` syntax pins it onto the file concerned, in the diff
/// view. Elsewhere it is a line like any other — the same information, without the encoding that
/// would be of use to nobody.
fn annotate(file: Option<&str>, message: impl std::fmt::Display) {
    warnings().push(serde_json::json!({ "file": file, "message": message.to_string() }));
    if ui::json() {
        ui::warn(message);
        return;
    }
    if std::env::var("GITHUB_ACTIONS").is_ok_and(|value| value == "true") {
        match file {
            Some(file) => println!("::warning file={file}::{message}"),
            None => println!("::warning::{message}"),
        }
        return;
    }
    ui::warn(message);
}

/// What the manifest declares and the platform is withdrawing.
///
/// The identifiers are compared with the contract's, never interpreted: the CLI does not know what
/// `core.storage` is, it only knows that the module names it and that the registry announces it as
/// going away. That is what lets a deprecation travel without a new version of the CLI.
fn warn_about_withdrawn(manifest: &ModuleManifest, catalogue: &[Withdrawn]) -> usize {
    let declared = declared_ids(manifest);

    let mut found = 0;
    for entry in catalogue {
        if !declared.iter().any(|name| name == &entry.id) {
            continue;
        }
        found += 1;
        let replacement = match &entry.replacement {
            Some(replacement) => format!(" — use {replacement}"),
            None => String::new(),
        };
        annotate(
            Some(MODULE_MANIFEST),
            format!(
                "{} `{}` is deprecated since {}{replacement}: {}",
                entry.subject, entry.id, entry.since, entry.note
            ),
        );
    }
    found
}

/// Everything the manifest names that the platform could withdraw from under its feet.
///
/// Kept apart from the rendering so that it can be checked: this list is what decides whether a
/// module is affected, and leaving a single category out of it would make the warning silent
/// exactly where it counts.
fn declared_ids(manifest: &ModuleManifest) -> Vec<String> {
    manifest
        .capabilities
        .required
        .iter()
        .chain(manifest.capabilities.provided.iter())
        .map(|capability| capability.as_str().to_string())
        .chain(
            manifest
                .capabilities
                .optional
                .iter()
                .map(|capability| capability.id.as_str().to_string()),
        )
        .chain(manifest.connectors.builtin.iter().cloned())
        .collect()
}

/// One entry of the deprecations contract, as the registry renders it.
#[derive(Debug, serde::Deserialize)]
struct Withdrawn {
    id: String,
    subject: String,
    since: String,
    #[serde(default)]
    replacement: Option<String>,
    #[serde(default)]
    note: String,
}

/// The deprecations contract for this version of the SDK.
///
/// On its own dedicated route rather than by reading all of the version's contracts: `ci check`
/// runs on every build, and downloading the manifest schema and the SDUI primitives to read an
/// often empty list would be paying a lot for a cheap question.
///
/// A version the registry does not know — an SDK compiled from a branch, never published —
/// returns a 404. That is not a defect of the module: there is nothing to say, so we stay quiet.
async fn withdrawn(sdk_version: &str) -> Result<Vec<Withdrawn>> {
    let base = crate::profile::api_url(None);
    let response = crate::http::client()
        .get(format!(
            "{base}/registry/v1/sdk-releases/{sdk_version}/deprecations"
        ))
        .send()
        .await?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(Vec::new());
    }
    let body: serde_json::Value = response.json().await?;
    let entries = body
        .get("deprecations")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    Ok(serde_json::from_value(entries).unwrap_or_default())
}

pub(crate) fn emit_outputs(pairs: &[(&str, &str)]) -> Result<()> {
    let Ok(path) = std::env::var("GITHUB_OUTPUT") else {
        return Ok(());
    };
    use std::io::Write as _;
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(&path)
        .with_context(|| format!("open {path}"))?;
    for (key, value) in pairs {
        writeln!(file, "{key}={value}").with_context(|| format!("write {key} to {path}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The lock of a repository that follows a branch: the version is the crate's, the revision
    /// is that of the commit being compiled.
    const GIT_LOCK: &str = r#"
[[package]]
name = "serde"
version = "1.0.0"

[[package]]
name = "portaki-sdk"
version = "2.2.0"
source = "git+https://github.com/PortakiApp/portaki-sdk.git?branch=main#28d522da69b70627f78123d9c42475cb7c595c46"
dependencies = [
 "base64",
]
"#;

    const REGISTRY_LOCK: &str = r#"
[[package]]
name = "portaki-sdk"
version = "2.3.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
"#;

    #[test]
    fn a_git_dependency_still_yields_a_published_version() {
        let resolved = read_locked_sdk(GIT_LOCK).expect("portaki-sdk dans le lock");

        assert_eq!(resolved.version, "2.2.0");
        assert_eq!(
            resolved.rev.as_deref(),
            Some("28d522da69b70627f78123d9c42475cb7c595c46")
        );
        // The key feeds `cargo install portaki-cli@<key>`: it carries nothing but the version.
        assert_eq!(resolved.cache_key(), "2.2.0");
    }

    #[test]
    fn a_registry_dependency_has_no_revision() {
        let resolved = read_locked_sdk(REGISTRY_LOCK).expect("portaki-sdk dans le lock");

        assert_eq!(resolved.cache_key(), "2.3.0");
        assert!(resolved.rev.is_none());
    }

    #[test]
    fn a_lock_without_the_sdk_says_so() {
        assert!(read_locked_sdk("[[package]]\nname = \"serde\"\nversion = \"1.0.0\"\n").is_none());
    }

    /// Compared as numbers: lexicographically, `2.9.0` would pass for later than `2.10.0`.
    /// The shared foundation forces a full rebuild — but a CI file is not part of it, otherwise
    /// the slightest touch-up to a workflow would set off twenty-one publications.
    #[test]
    fn only_the_shared_workspace_fans_out() {
        assert!(touches_shared("Cargo.lock"));
        assert!(touches_shared("Cargo.toml"));
        assert!(touches_shared(".cargo/config.toml"));
        assert!(touches_shared("rust-toolchain.toml"));

        assert!(!touches_shared(".github/workflows/ci.yml"));
        assert!(!touches_shared("modules/weather/Cargo.toml"));
        assert!(!touches_shared("README.md"));
    }

    /// A layout is declared nowhere: the manifest is what reveals it.
    #[test]
    fn both_repository_layouts_are_recognised() {
        let single = tempfile::tempdir().unwrap();
        std::fs::write(single.path().join(MODULE_MANIFEST), r#"{"id":"weather"}"#).unwrap();
        assert_eq!(discover(single.path()).unwrap().len(), 1);

        let many = tempfile::tempdir().unwrap();
        for name in ["weather", "nuki"] {
            let directory = many.path().join(MODULES_DIR).join(name);
            std::fs::create_dir_all(&directory).unwrap();
            std::fs::write(directory.join(MODULE_MANIFEST), r#"{"id":"x"}"#).unwrap();
        }
        assert_eq!(discover(many.path()).unwrap(), vec!["nuki", "weather"]);
    }

    /// The document comes from the registry: its shape is a contract, not a detail.
    #[test]
    fn the_registry_document_reads_as_the_cli_expects() {
        let document = serde_json::json!([
            {
                "id": "core.storage",
                "subject": "capability",
                "since": "2.4.0",
                "replacement": "core.kv",
                "note": "typed repositories replace raw storage"
            },
            { "id": "kv.list", "subject": "hostOp", "since": "2.4.0", "note": "never scaled" }
        ]);

        let parsed: Vec<Withdrawn> = serde_json::from_value(document).unwrap();

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].replacement.as_deref(), Some("core.kv"));
        // With no replacement, the key is absent from the document — it must not make the read
        // fail, or a warning would disappear instead of being shown.
        assert!(parsed[1].replacement.is_none());
        assert_eq!(parsed[1].since, "2.4.0");
    }

    /// A category forgotten here would make the warning silent exactly where it counts.
    #[test]
    fn every_kind_of_declaration_is_looked_at() {
        let raw = serde_json::json!({
            "manifestVersion": "1",
            "id": "weather",
            "version": "1.0.0",
            "displayName": "k",
            "description": "k",
            "author": { "name": "x" },
            "uiSchema": { "host": "1", "guest": "1" },
            "capabilities": {
                "required": ["core.storage"],
                "optional": [
                    { "id": "core.guests.notifications", "purpose_key": "k", "fallback_key": "k" }
                ],
                "provided": ["access.smart_lock"]
            },
            "connectors": { "builtin": ["open-weather"], "custom": [] },
            "entities": [],
            "surfaces": {},
            "queries": [],
            "commands": [],
            "events": {},
            "i18n": { "default": "fr-FR", "supported": ["fr-FR"] }
        });
        let manifest: ModuleManifest = serde_json::from_value(raw).expect("manifeste lisible");

        let declared = declared_ids(&manifest);

        for expected in [
            "core.storage",
            "core.guests.notifications",
            "access.smart_lock",
            "open-weather",
        ] {
            assert!(
                declared.iter().any(|id| id == expected),
                "manque {expected}"
            );
        }
    }

    /// The template used to take the id from the repository name: wrong as soon as one repository
    /// holds several, and fragile even with a single one. It is declared, so it is read.
    #[test]
    fn the_module_id_comes_from_the_manifest_not_the_directory() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(
            root.path().join(MODULE_MANIFEST),
            r#"{"id":"access-guide","version":"0.3.2"}"#,
        )
        .unwrap();

        assert_eq!(read_module_id(root.path()).unwrap(), "access-guide");
    }

    #[test]
    fn a_manifest_without_an_id_says_so() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join(MODULE_MANIFEST), r#"{"version":"1.0.0"}"#).unwrap();

        assert!(read_module_id(root.path()).is_err());
    }

    /// A directory under `modules/` with no manifest is not a module — `target/`, for instance.
    #[test]
    fn a_directory_without_a_manifest_is_not_a_module() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join(MODULES_DIR).join("target")).unwrap();

        assert!(discover(root.path()).unwrap().is_empty());
    }
}
