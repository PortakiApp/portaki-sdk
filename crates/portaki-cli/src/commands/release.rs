//! `portaki release` — publish a version to the Portaki registry.
//!
//! Tests (the conformance battery of `portaki_test_utils::conformance!()` included) → `--release`
//! build → packing → right to push asked of the registry → push into **its** OCI repository →
//! signature → announcement → public listing. A failing test stops everything before the build; a
//! failing signature, before the announcement.
//!
//! The right to push comes from the registry (`POST /registry/v1/publications/push-token`): short,
//! limited to this module and this version, for the one `modules/<id>` repository of the OCI host
//! the answer names. Nothing else authorises a push — not `docker login`, not a GitHub token.
//!
//! Who publishes: from a workstation, the `portaki login` session; in a GitHub Actions job with
//! `id-token: write`, the job's OIDC token, exchanged for a single-use credential (one per step:
//! right to push, announcement, listing).
//!
//! The signature is on by default. From a workstation, it is the author who attests (`cosign
//! attest`, GitHub identity); in CI, it is [`Mode::CiRelease`] that attests the workflow's
//! provenance and the dependency audit. `--no-sign` publishes without a signature: the version
//! will then only run in the sandbox.
//!
//! In CI, two jobs: `portaki ci build` runs the module's code (tests, `build.rs`) with no rights
//! at all; `portaki ci release` pushes what it produced, with the rights, running nothing of the
//! module — not even cargo: id, version and SDK are read from `publish-manifest.json`, the sources
//! and `Cargo.lock`.
//!
//! Set `PORTAKI_PUBLISH_VERSION` (e.g. from CI git tag `*-vX.Y.Z`) to fail fast if
//! `publish-manifest.json` version does not match.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Parser;

use crate::commands::link;
use crate::{auth, oci, oidc, sign, ui, workspace};

#[derive(Debug, Clone, Parser)]
/// Arguments for `portaki release`.
pub struct ReleaseArgs {
    /// Run the tests, build and package without pushing or announcing anything.
    #[arg(long)]
    pub dry_run: bool,
    /// Release channel at the Portaki registry — `stable` needs SDK 8.0.0 or later.
    #[arg(long, default_value = "stable", value_parser = ["preview", "stable"])]
    pub channel: String,
    /// Publish without a signature. The version then never runs in production (any channel):
    /// it stays usable in the sandbox.
    #[arg(long)]
    pub no_sign: bool,
    /// In a repository holding several modules, the one to publish.
    #[arg(long, conflicts_with = "all")]
    pub module: Option<String>,
    /// Publish every module of the repository, each on its own.
    #[arg(long)]
    pub all: bool,
    /// A line of what is new in this version, shown to hosts on an older one (repeatable, at
    /// most 5 per language). `fr:Code clavier` tags its language; untagged, --notes-lang.
    /// Without it for a language, the version's section of CHANGELOG.<lang>.md (CHANGELOG.md
    /// in English). A note that reads like a commit message (`chore:`, `fix:`, `bump`…) is
    /// refused; in a file, such a line is dropped with a warning.
    #[arg(long = "notes", value_name = "[LANG:]LINE")]
    pub notes: Vec<String>,
    /// Language of untagged notes and texts. Defaults to the first language of listing.json
    /// (`publishedLangs`), else `fr` — the one the registry requires. CHANGELOG.md stays English.
    #[arg(long)]
    pub notes_lang: Option<String>,
    /// Fail when a stable version lands as a draft, invisible to hosts until its notes are
    /// completed. Off by default: the version is in the registry either way.
    #[arg(long)]
    pub require_available: bool,
    /// Why this version adds a permission, shown to hosts under the platform's sentence
    /// (repeatable: one per permission and language), e.g. `email=fr:Pour envoyer le code`.
    #[arg(long = "permission-reason", value_name = "PERMISSION=[LANG:]TEXT")]
    pub permission_reasons: Vec<String>,
    /// The host has something to do after updating (reconnect, reconfigure…).
    #[arg(long)]
    pub host_action_required: bool,
    /// What the host has to do, one per language (implies --host-action-required).
    #[arg(long = "host-action", value_name = "[LANG:]TEXT")]
    pub host_action: Vec<String>,
}

/// Where the publication runs, and therefore what it is allowed to execute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    /// A workstation: tests, build, author signature.
    Local,
    /// `portaki ci build`, the job with no rights: tests, build, packing — nothing is pushed.
    CiBuild,
    /// `portaki ci release`, the job that holds the rights: the artifact from `ci build`, pushed,
    /// attested with the workflow's provenance and `audit`, announced. Nothing of the module runs.
    CiRelease { audit: Option<PathBuf> },
}

/// A layer's size on disk, or zero when it cannot be read — the list is a report, not a gate.
fn layer_size(path: &Path) -> u64 {
    std::fs::metadata(path).map(|meta| meta.len()).unwrap_or(0)
}

/// `1 layer`, `5 layers`.
fn plural(count: usize, noun: &str) -> String {
    if count == 1 {
        format!("{count} {noun}")
    } else {
        format!("{count} {noun}s")
    }
}

/// Runs `portaki release`.
pub async fn run(args: ReleaseArgs) -> Result<()> {
    run_as(args, Mode::Local).await
}

/// `portaki release`, `portaki ci build` or `portaki ci release`, for each module aimed at.
pub async fn run_as(args: ReleaseArgs, mode: Mode) -> Result<()> {
    match mode {
        Mode::Local => ui::header(
            "portaki release",
            &crate::tr!(
                "The gate of portaki check, then push to Portaki's registry, sign, and announce.",
                "La porte de portaki check, puis poussée au registre Portaki, signature et annonce."
            ),
        ),
        Mode::CiBuild => ui::header(
            "portaki ci build",
            "Test, build and package — nothing is pushed from this job.",
        ),
        Mode::CiRelease { .. } => ui::header(
            "portaki ci release",
            "Push what ci build produced, sign it as this workflow, announce it.",
        ),
    }

    // One module after another, each with its own right to push and its own digest: a refusal
    // does not stop the ones that follow, and the exit code says whether there was one.
    let chosen = workspace::resolve(args.module.as_deref(), Some(args.all))?;
    let mut outcomes = Vec::with_capacity(chosen.len());
    let mut results = Vec::with_capacity(chosen.len());
    for member in &chosen {
        if chosen.len() > 1 {
            ui::rule(&member.id);
        }
        workspace::enter(member)?;
        *result() = serde_json::json!({
            "id": member.id, "version": null, "channel": args.channel, "state": "failed",
            "digest": null, "reference": null, "missing": [], "url": null, "error": null,
        });
        let outcome = run_in(&member.root, &args, &mode).await;
        if let Err(failure) = &outcome {
            note("error", format!("{failure:#}"));
        }
        results.push(result().take());
        outcomes.push((member.id.clone(), outcome));
    }
    if mode != Mode::Local {
        ci_outputs(&results)?;
    }
    if ui::json() {
        if results
            .iter()
            .all(|result| result["state"] == "already-published")
        {
            crate::exit::nothing_to_do();
        }
        ui::emit(&serde_json::json!({ "schemaVersion": 1, "modules": results }));
    }
    conclude(outcomes)
}

/// What a following workflow step reads from `GITHUB_OUTPUT`: `results` for all of them, and the
/// fields laid out flat when there is only one module — the case of the release action.
fn ci_outputs(results: &[serde_json::Value]) -> Result<()> {
    let mut pairs = vec![("results".to_string(), serde_json::to_string(results)?)];
    if let [only] = results {
        for (key, field) in [
            ("id", "id"),
            ("version", "version"),
            ("outcome", "state"),
            ("digest", "digest"),
            ("reference", "reference"),
        ] {
            let value = only[field].as_str().unwrap_or_default().to_string();
            pairs.push((key.to_string(), value));
        }
    }
    let pairs: Vec<(&str, &str)> = pairs
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    crate::commands::ci::emit_outputs(&pairs)
}

/// The current module's result, for `--json`: recorded where each fact becomes known rather than
/// handed on from step to step. Modules are published one at a time, never together.
static RESULT: std::sync::Mutex<serde_json::Value> = std::sync::Mutex::new(serde_json::Value::Null);

fn result() -> std::sync::MutexGuard<'static, serde_json::Value> {
    RESULT
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn note(key: &str, value: impl Into<serde_json::Value>) {
    if let Some(fields) = result().as_object_mut() {
        fields.insert(key.to_string(), value.into());
    }
}

/// The `module_not_linked` refusal from the OIDC exchange, if it is in the chain.
fn not_linked(failure: &anyhow::Error) -> Option<&oidc::Refused> {
    failure
        .chain()
        .find_map(|cause| cause.downcast_ref::<oidc::Refused>())
        .filter(|refused| refused.code == "module_not_linked")
}

/// The modules refused for want of a link, in the run's own order.
///
/// The CLI cannot list the links — the API that gives them wants a developer token, and a CI has
/// only its OIDC token. So it is this run's refusals that make up the list.
fn unlinked(outcomes: &[(String, Result<()>)]) -> Vec<String> {
    outcomes
        .iter()
        .filter(|(_, outcome)| outcome.as_ref().err().and_then(not_linked).is_some())
        .map(|(id, _)| id.clone())
        .collect()
}

/// One result per module, the link that links all the unlinked ones in one go, and a failure if
/// even a single module did not get through.
fn conclude(outcomes: Vec<(String, Result<()>)>) -> Result<()> {
    let unlinked = unlinked(&outcomes);
    // The Dépôt page comes from the registry, in the refusal of the first unlinked module.
    let page = outcomes
        .iter()
        .find_map(|(_, outcome)| outcome.as_ref().err().and_then(not_linked))
        .and_then(|refused| refused.link_url.clone());
    let total = outcomes.len();
    let mut failed = 0;
    let mut status = 403;

    if total > 1 {
        ui::section("results");
    }
    for (id, outcome) in outcomes {
        let Err(failure) = outcome else {
            if total > 1 {
                ui::success(crate::tr!("{id} published", "{id} publié"));
            }
            continue;
        };
        failed += 1;
        if let Some(refused) = not_linked(&failure) {
            status = refused.status;
            if total > 1 {
                ui::failure(crate::tr!(
                    "{id} — not linked to any repository",
                    "{id} — lié à aucun dépôt"
                ));
            }
        } else if total == 1 {
            // A single module: the failure travels up as it stands, as before.
            return Err(failure);
        } else {
            ui::failure(format!("{id} — {failure:#}"));
        }
    }

    if let Some(first) = unlinked.first() {
        ui::blank();
        ui::failure(crate::tr!(
            "{status} module_not_linked — « {first} » is not linked to any repository",
            "{status} module_not_linked — « {first} » n'est lié à aucun dépôt"
        ));
        ui::detail(crate::tr!(
            "Modules not linked in this repository: {}",
            "Modules non liés dans ce dépôt : {}",
            unlinked.join(", ")
        ));
        if let Some(page) = &page {
            ui::detail(crate::tr!(
                "→ Link them at once: {}",
                "→ Liez-les en une fois : {}",
                link::with_also(page, &unlinked[1..])
            ));
        }
        ui::blank();
    }

    match failed {
        0 => Ok(()),
        _ if total == 1 => anyhow::bail!(crate::tr!(
            "{} was not published",
            "{} n'a pas été publié",
            unlinked.join(", ")
        )),
        _ => anyhow::bail!(crate::tr!(
            "{failed} of {total} modules failed",
            "{failed} modules sur {total} en échec"
        )),
    }
}

/// The module's public listing, versioned next to `portaki.module.json`.
const LISTING: &str = "listing.json";

/// The module's listing, read and checked before any publication: a broken listing discovered
/// after the push would leave a published artifact and a storefront lagging behind.
///
/// The content goes out as it stands — it is the registry that validates its fields.
fn read_listing(module_root: &Path) -> Result<Option<serde_json::Value>> {
    let path = module_root.join(LISTING);
    let raw = match std::fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(failure) if failure.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(failure) => return Err(failure).with_context(|| format!("read {}", path.display())),
    };
    let listing: serde_json::Value = serde_json::from_str(&raw)
        .with_context(|| format!("{LISTING} is not valid JSON — fix it before publishing"))?;
    if !listing.is_object() {
        anyhow::bail!("{LISTING} must hold a JSON object — fix it before publishing");
    }
    Ok(Some(listing))
}

/// How far the publication got.
#[derive(Debug, PartialEq, Eq)]
enum Landed {
    DryRun,
    /// The version is in the registry — announced just now, or already there.
    InRegistry(String),
}

/// The version is already in the registry: the publication stops before pushing (ADR-0005).
#[derive(Debug)]
struct AlreadyInRegistry {
    id: String,
    version: String,
    digest: String,
}

impl std::fmt::Display for AlreadyInRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} is already in the registry ({}) — publications are immutable: bump the \
             version to ship a change",
            self.id, self.version, self.digest
        )
    }
}

impl std::error::Error for AlreadyInRegistry {}

/// In the registry, but as a draft, while `--require-available` asked for a visible version.
#[derive(Debug)]
struct DraftRefused {
    id: String,
    version: String,
}

impl std::fmt::Display for DraftRefused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} is in the registry as a draft, invisible to hosts (--require-available) — \
             complete it at the link above",
            self.id, self.version
        )
    }
}

impl std::error::Error for DraftRefused {}

/// What becomes of the listing, depending on where the publication stopped.
#[derive(Debug, PartialEq, Eq)]
enum ListingPlan<'a> {
    Send(&'a str),
    WouldSend,
    Skip,
}

/// The listing goes out as soon as the version is in the registry — including one already
/// published, otherwise a corrected listing would wait for the next release.
fn listing_plan(landed: &Result<Landed>) -> ListingPlan<'_> {
    match landed {
        Ok(Landed::InRegistry(id)) => ListingPlan::Send(id),
        Ok(Landed::DryRun) => ListingPlan::WouldSend,
        Err(failure) => {
            if let Some(already) = failure.downcast_ref::<AlreadyInRegistry>() {
                return ListingPlan::Send(&already.id);
            }
            match failure.downcast_ref::<DraftRefused>() {
                Some(draft) => ListingPlan::Send(&draft.id),
                None => ListingPlan::Skip,
            }
        }
    }
}

/// A version already in the registry is not a failure: nothing is pushed, and what comes next —
/// the listing — goes out all the same.
///
/// It used to be an error, and that is what turned red a run replayed only to push a corrected
/// listing. The warning goes out on stdout, where the v1 workflows looked for it; `ci release`
/// also says it in `GITHUB_OUTPUT` (`outcome=already-published`).
fn settle(landed: Result<Landed>) -> Result<Landed> {
    match landed {
        Err(failure) => match failure.downcast_ref::<AlreadyInRegistry>() {
            Some(already) => {
                note("state", "already-published");
                note("version", already.version.clone());
                note("digest", already.digest.clone());
                ui::warn_on_stdout(already);
                Ok(Landed::InRegistry(already.id.clone()))
            }
            None => Err(failure),
        },
        landed => landed,
    }
}

/// The publication of the module at `module_root`, then its public listing.
async fn run_in(module_root: &Path, args: &ReleaseArgs, mode: &Mode) -> Result<()> {
    let base = crate::profile::api_url(None);
    let Some(listing) = read_listing(module_root)? else {
        return settle(release(module_root, args, mode, &base).await).map(|_| ());
    };
    let landed = settle(release(module_root, args, mode, &base).await);
    let sent = match listing_plan(&landed) {
        ListingPlan::Send(id) => send_listing(&base, &args.channel, id, &listing).await,
        ListingPlan::WouldSend => {
            ui::field("listing", format!("would be sent ({LISTING})"));
            Ok(())
        }
        ListingPlan::Skip => Ok(()),
    };
    match (landed, sent) {
        (Err(failure), Ok(())) => Err(failure),
        (Err(failure), Err(listing)) => Err(anyhow::anyhow!("{failure:#}\n{listing:#}")),
        (Ok(_), sent) => sent,
    }
}

/// Sends the listing to the registry. It replaces the one edited in the dashboard: the repository
/// is what counts.
///
/// A CI credential is single-use and the announcement consumed its own: we ask for another.
async fn send_listing(
    base: &str,
    channel: &str,
    module_id: &str,
    listing: &serde_json::Value,
) -> Result<()> {
    let outcome = match credential(base, module_id, channel).await? {
        Credential::Ci(token) => put_listing(base, module_id, listing, &token).await?,
        Credential::Person(token) => {
            let first = put_listing(base, module_id, listing, &token).await?;
            if first == Outcome::Unauthorized {
                put_listing(
                    base,
                    module_id,
                    listing,
                    &auth::refresh(base, &token).await?,
                )
                .await?
            } else {
                first
            }
        }
    };
    // Not a failure: the publication stands, but the repository's listing changed nothing (it is
    // managed in the console) — say so plainly rather than a "sent" that would suggest otherwise.
    if let Outcome::Ignored(message) = &outcome {
        ui::warn(format!("listing not applied — {message}"));
        return Ok(());
    }
    let verdict = listing_verdict(module_id, outcome);
    match &verdict {
        Ok(()) => ui::field("listing", "sent"),
        Err(failure) => ui::field("listing", format!("{failure:#}")),
    }
    verdict
}

async fn put_listing(
    base: &str,
    module_id: &str,
    listing: &serde_json::Value,
    token: &str,
) -> Result<Outcome> {
    let response = crate::http::client()
        .put(format!(
            "{}/registry/v1/modules/{module_id}/listing",
            base.trim_end_matches('/')
        ))
        .bearer_auth(token)
        .json(listing)
        .send()
        .await
        .context("send the public listing to the registry")?;

    Ok(classify(
        response.status().as_u16(),
        &response.text().await.unwrap_or_default(),
    ))
}

/// A refused listing does not undo the publication: it fails the run, with the reason.
fn listing_verdict(module_id: &str, outcome: Outcome) -> Result<()> {
    match outcome {
        Outcome::Published | Outcome::Draft { .. } | Outcome::Ignored(_) => Ok(()),
        Outcome::Refused {
            status,
            code,
            message,
        } => anyhow::bail!(
            "the registry refused the listing of {module_id} ({status} {code}): {message} — \
             the publication itself stands; fix {LISTING} and replay"
        ),
        Outcome::Unauthorized | Outcome::AlreadyPublished => anyhow::bail!(
            "the registry refused the token for the listing of {module_id} — run portaki login, \
             or replay the job; the publication itself stands"
        ),
    }
}

/// The publication of the module at `module_root`, up to the announcement.
async fn release(
    module_root: &Path,
    args: &ReleaseArgs,
    mode: &Mode,
    base: &str,
) -> Result<Landed> {
    let artifact_dir = module_root.join("target/portaki");
    let lang = notes_lang(args, module_root);
    // Read before any build: a malformed flag discovered at announcement time would leave a
    // pushed artifact and an unannounced version.
    let notes = crate::changelog::release_notes(
        &args.permission_reasons,
        args.host_action_required,
        &args.host_action,
        &lang,
    )?;

    // Before any build: a refusal discovered after the push would leave an unsigned artifact.
    let signs = !args.no_sign && *mode != Mode::CiBuild;
    let cosign = sign::cosign_binary();
    if signs && !args.dry_run {
        match mode {
            Mode::Local => sign::refuse_in_ci(sign::in_ci())?,
            _ if !oidc::available() => anyhow::bail!(
                "signing in CI needs the job's OIDC token — add `permissions: id-token: write` \
                 to the job (--no-sign publishes unsigned, for the sandbox only)"
            ),
            _ => {}
        }
        sign::check_cosign(&cosign)?;
    }

    if let Mode::CiRelease { audit } = mode {
        // The job that holds the rights runs nothing of the module: not its tests, not its build,
        // not cargo. The artifact comes from `ci build`; it picks neither its name nor its SDK.
        ui::skipped("build and tests skipped — portaki ci build ran them, in a job without rights");
        artifact_matches_sources(module_root, &artifact_dir)?;
        sdk_matches_lock(module_root, &artifact_dir)?;
        if let Some(audit) = audit.as_deref().filter(|_| signs) {
            anyhow::ensure!(
                audit.is_file(),
                "no cargo audit report at {} — produce it in this job, from Cargo.lock",
                audit.display()
            );
        }
    } else {
        // The gate of `portaki check`, exactly as it is: what it lets through, and only that.
        // Before any right to push — a refusal found afterwards would leave a pushed artifact.
        crate::commands::check::gate(module_root, &args.channel, false, &[])
            .await
            .context(crate::tr!(
                "the gate before publishing — portaki check runs the same",
                "la porte avant publication — portaki check joue la même"
            ))?;
        ui::blank();
    }

    stamp_changelog(module_root, &artifact_dir, args, &lang)?;
    // Before the push: the registry would refuse the announcement, but the artifact would already
    // have been pushed.
    crate::commands::lint::assert_sdk_version(
        &oci::pack::publish_manifest_path(&artifact_dir),
        &args.channel,
    )?;

    let packing = ui::step("packing the OCI artifact");
    // The layer list the push would send, assembled here rather than at push time: it is what
    // says the wasm exists. A dry run that skipped it answered a question it had not checked.
    let layers = oci::pack::collect_push_layers(module_root, &artifact_dir).map_err(|failure| {
        packing.abandon();
        failure
    })?;
    assert_publish_version_matches_env(module_root, &artifact_dir)?;
    packing.done(format!("packed {}", plural(layers.len(), "layer")));
    for layer in &layers {
        ui::detail(format!(
            "{}  {}",
            layer
                .path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_default(),
            ui::bytes(layer_size(&layer.path))
        ));
    }

    let coords = oci::pack::read_module_coordinates(module_root, &artifact_dir)?;
    note("version", coords.version.clone());
    if args.dry_run || *mode == Mode::CiBuild {
        note("state", "dry-run");
        ui::success(crate::tr!(
            "nothing was pushed, nothing was announced",
            "rien n'a été poussé, rien n'a été annoncé"
        ));
        ui::field("artifact", artifact_dir.display());
        if *mode == Mode::CiBuild {
            ui::advice("hand target/portaki and the wasm to the job that runs portaki ci release");
        } else {
            ui::advice(crate::tr!(
                "drop --dry-run to push these layers and announce the version",
                "retirez --dry-run pour pousser ces couches et annoncer la version"
            ));
        }
        ui::blank();
        return Ok(Landed::DryRun);
    }

    // Asked before pushing, not discovered afterwards. A publication is immutable (ADR-0005):
    // republishing would rewrite the OCI tag, which would no longer point at the artifact the
    // catalogue references.
    refuse_if_already_published(base, &coords).await?;

    let pushing = ui::step(crate::tr!(
        "asking the registry for the right to push",
        "demande du droit de push au registre"
    ));
    let grant = push_grant(base, &coords, &args.channel)
        .await
        .map_err(|failure| {
            pushing.abandon();
            failure
        })?;
    pushing.say(crate::tr!(
        "pushing to {}",
        "poussée vers {}",
        grant.reference
    ));
    let pushed = oci::push_artifact(module_root, &artifact_dir, &grant)
        .await
        .map_err(|failure| {
            pushing.abandon();
            failure
        })?;
    pushing.done(crate::tr!(
        "pushed to {}",
        "poussé vers {}",
        grant.reference
    ));
    ui::field("digest", &pushed.digest);
    note("state", "pushed");
    note("digest", pushed.digest.clone());
    note("reference", pushed.artifact_ref());

    // The signature comes before the announcement: if it fails, nothing is announced.
    match mode {
        _ if !signs => ui::warn(crate::tr!(
            "unsigned — this version will never run in production (a signature is required); it \
             stays usable in the sandbox. Drop --no-sign, or publish from CI.",
            "non signée — cette version ne s'exécutera jamais en production (signature exigée) ; \
             elle reste utilisable en sandbox. Retirez --no-sign, ou publiez depuis la CI."
        )),
        Mode::CiRelease { audit } => {
            let audited = sign::attest_ci(&cosign, &pushed, &grant, audit.as_deref())
                .context("sign the pushed artifact — nothing was announced")?;
            ui::success("signed with this workflow's identity (provenance)");
            if !audited {
                ui::warn(
                    "no cargo audit report — the registry will show this version as not audited",
                );
            }
        }
        _ => {
            let email = sign::sign(&cosign, &pushed, &grant, &coords)
                .await
                .context("sign the pushed artifact — nothing was announced")?;
            ui::success(crate::tr!(
                "signed by {email} (outside CI)",
                "signé par {email} (hors CI)"
            ));
        }
    }

    announce(base, args, &coords, &pushed, &notes).await?;
    Ok(Landed::InRegistry(coords.id))
}

/// The language of the untagged `--notes` and texts.
fn notes_lang(args: &ReleaseArgs, module_root: &Path) -> String {
    args.notes_lang
        .clone()
        .unwrap_or_else(|| crate::changelog::default_lang(module_root))
}

/// The artifact comes from a job that ran the module's code: it does not get to choose the name
/// it goes out under. Without this check, a booby-trapped module would produce a
/// `publish-manifest.json` in another module's name, and the publishing job would push it under
/// that name.
fn artifact_matches_sources(module_root: &Path, artifact_dir: &Path) -> Result<()> {
    let sources = oci::pack::read_source_coordinates(module_root)?;
    let artifact = oci::pack::read_module_coordinates(module_root, artifact_dir)?;
    if artifact.id != sources.id || artifact.version != sources.version {
        anyhow::bail!(
            "the prebuilt artifact is {} {}, the sources are {} {} — refusing to publish it",
            artifact.id,
            artifact.version,
            sources.id,
            sources.version
        );
    }
    Ok(())
}

/// The SDK the artifact declares is the one `Cargo.lock` resolves: read from the lock, without
/// cargo — a `rust-toolchain.toml` or a `.cargo/config.toml` of the module's decides nothing here.
fn sdk_matches_lock(module_root: &Path, artifact_dir: &Path) -> Result<()> {
    use crate::commands::ci::{find_lockfile, read_locked_sdk};
    let path = oci::pack::publish_manifest_path(artifact_dir);
    let declared = std::fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .and_then(|manifest| Some(manifest.get("sdkVersion")?.as_str()?.to_string()));
    let lock = find_lockfile(module_root).context(
        "no Cargo.lock here or above — commit it: it says which SDK the module is built on",
    )?;
    let text =
        std::fs::read_to_string(&lock).with_context(|| format!("read {}", lock.display()))?;
    let locked = read_locked_sdk(&text)
        .with_context(|| format!("{} carries no portaki-sdk", lock.display()))?;
    if declared.as_deref() != Some(locked.version.as_str()) {
        anyhow::bail!(
            "the prebuilt artifact declares portaki-sdk {}, Cargo.lock resolves {} — refusing to \
             publish it",
            declared.as_deref().unwrap_or("nothing"),
            locked.version
        );
    }
    Ok(())
}

/// Writes `changelog` into `publish-manifest.json`: `--notes` per language, otherwise the
/// version's section of `CHANGELOG[.<lang>].md`. Read from the sources, never from what a build
/// produced.
fn stamp_changelog(
    module_root: &Path,
    artifact_dir: &Path,
    args: &ReleaseArgs,
    lang: &str,
) -> Result<()> {
    let coords = oci::pack::read_module_coordinates(module_root, artifact_dir)?;
    let lines = crate::changelog::lines(&args.notes, lang, module_root, &coords.version)?;
    if lines.is_empty() {
        return Ok(());
    }
    let path = oci::pack::publish_manifest_path(artifact_dir);
    let raw = std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let stamped = crate::changelog::stamp(&raw, &lines)?;
    std::fs::write(&path, stamped).with_context(|| format!("write {}", path.display()))?;
    let langs: std::collections::BTreeSet<&str> = lines
        .iter()
        .flat_map(|line| line.keys().map(String::as_str))
        .collect();
    ui::field(
        "changelog",
        format!(
            "{} ({})",
            plural(lines.len(), "line"),
            langs.into_iter().collect::<Vec<_>>().join(", ")
        ),
    );
    Ok(())
}

/// A published version is not published again.
///
/// Asked before the push: the announcement would refuse it, but the tag would already have been
/// rewritten. The catalogue is public: the question costs neither a token nor a right.
///
/// If it cannot be reached, we carry on. The right to push refuses a published version anyway.
async fn refuse_if_already_published(
    base: &str,
    coords: &oci::pack::ModuleCoordinates,
) -> Result<()> {
    let Some(published) = published_digest(base, &coords.id, &coords.version).await else {
        return Ok(());
    };
    Err(AlreadyInRegistry {
        id: coords.id.clone(),
        version: coords.version.clone(),
        digest: published,
    }
    .into())
}

/// A version in the catalogue, as the registry renders it.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PublishedVersion {
    digest: String,
    version: String,
}

/// The digest published for this version, if there is one.
async fn published_digest(base: &str, module_id: &str, version: &str) -> Option<String> {
    let response = crate::http::client()
        .get(format!(
            "{}/registry/v1/modules/{module_id}/versions",
            base.trim_end_matches('/')
        ))
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    digest_of(
        &response.json::<Vec<PublishedVersion>>().await.ok()?,
        version,
    )
}

/// The selection on its own, kept apart from the network so that it can be checked.
fn digest_of(published: &[PublishedVersion], version: &str) -> Option<String> {
    published
        .iter()
        .find(|candidate| candidate.version == version)
        .map(|candidate| candidate.digest.clone())
}

/// The right to push this version, asked of the registry (`push-token`) with the same publisher
/// as the announcement: the session, or a credential exchanged for the job's OIDC token.
async fn push_grant(
    base: &str,
    coords: &oci::pack::ModuleCoordinates,
    channel: &str,
) -> Result<oci::PushGrant> {
    let body = serde_json::json!({
        "moduleId": coords.id,
        "version": coords.version,
        "channel": channel,
    });
    let (status, text) = match credential(base, &coords.id, channel).await? {
        Credential::Ci(token) => post_push_token(base, &body, &token).await?,
        Credential::Person(token) => {
            let first = post_push_token(base, &body, &token).await?;
            if first.0 == 401 {
                post_push_token(base, &body, &auth::refresh(base, &token).await?).await?
            } else {
                first
            }
        }
    };
    grant_from(status, &text)
}

async fn post_push_token(
    base: &str,
    body: &serde_json::Value,
    token: &str,
) -> Result<(u16, String)> {
    let response = crate::http::client()
        .post(format!(
            "{}/registry/v1/publications/push-token",
            base.trim_end_matches('/')
        ))
        .bearer_auth(token)
        .json(body)
        .send()
        .await
        .context("ask the registry for the right to push")?;
    Ok((
        response.status().as_u16(),
        response.text().await.unwrap_or_default(),
    ))
}

/// The registry's answer: the right to push, or the refusal with its stable code.
fn grant_from(status: u16, body: &str) -> Result<oci::PushGrant> {
    if (200..300).contains(&status) {
        return serde_json::from_str(body)
            .context("the registry returned an unreadable push grant");
    }
    match classify(status, body) {
        Outcome::Unauthorized => anyhow::bail!(
            "the registry refused the token for a push — run portaki login, or replay the job"
        ),
        Outcome::Refused {
            status,
            code,
            message,
        } => anyhow::bail!("the registry refused the push ({status} {code}): {message}"),
        _ => anyhow::bail!("the registry refused the push ({status}): {body}"),
    }
}

/// Announces the publication to the registry, renewing the token once on a 401.
///
/// Failing here leaves the artifact pushed and signed, but nothing is published: replaying
/// `release` (or the job) asks for a right to push again, pushes again, signs again and
/// announces.
async fn announce(
    base: &str,
    args: &ReleaseArgs,
    coords: &oci::pack::ModuleCoordinates,
    pushed: &oci::PushedArtifact,
    notes: &serde_json::Value,
) -> Result<()> {
    let announcing = ui::step(crate::tr!(
        "announcing {} to the registry",
        "annonce en {} au registre",
        args.channel
    ));
    let body = serde_json::json!({
        "moduleId": coords.id,
        "version": coords.version,
        "artifactRef": pushed.artifact_ref(),
        "digest": pushed.digest,
        "channel": args.channel,
        "releaseNotes": notes,
    });

    let outcome = match credential(base, &coords.id, &args.channel).await? {
        // A CI: the credential is single-use, so a 401 means consumed or expired. Replaying with
        // the same one would have no chance; a fresh exchange is needed.
        Credential::Ci(token) => post_publication(base, &body, &token).await?,
        Credential::Person(token) => {
            let first = post_publication(base, &body, &token).await?;
            if first == Outcome::Unauthorized {
                post_publication(base, &body, &auth::refresh(base, &token).await?).await?
            } else {
                first
            }
        }
    };

    match outcome {
        Outcome::Published | Outcome::Draft { .. } => {
            note("version", coords.version.clone());
            note("state", "published");
            if let Outcome::Draft { missing, url } = &outcome {
                note("state", "draft");
                note("missing", missing.clone());
                note("url", url.clone());
            }
            announcing.done(crate::tr!(
                "announced to the registry on {}",
                "annoncée au registre en {}",
                args.channel
            ));
            ui::field("module", format!("{} {}", coords.id, coords.version));
            ui::field("channel", &args.channel);
            ui::field("reference", pushed.artifact_ref());
            if let Outcome::Draft { missing, url } = &outcome {
                // Not a failure by default: the version is in the registry, waiting for its
                // notes. The CI stays green, and the author knows what to complete and where.
                ui::warn(crate::tr!(
                    "draft — invisible to hosts while it misses:",
                    "brouillon — invisible des hôtes tant qu'il manque :"
                ));
                for item in missing {
                    ui::detail(format!("- {item}"));
                }
                if let Some(url) = url {
                    ui::detail(crate::tr!("→ complete: {url}", "→ compléter : {url}"));
                }
                ui::next(&[(
                    &format!("portaki release notes {} --complete", coords.version),
                    &crate::tr!(
                        "complete the notes from here — --notes, or CHANGELOG.<lang>.md",
                        "compléter la version d'ici — --notes, ou CHANGELOG.<lang>.md"
                    ),
                )]);
                if args.require_available && args.channel == "stable" {
                    return Err(DraftRefused {
                        id: coords.id.clone(),
                        version: coords.version.clone(),
                    }
                    .into());
                }
            } else {
                ui::field("release", crate::tr!("available", "disponible"));
                ui::next(&[
                    (
                        &format!("portaki release status {}", coords.version),
                        &crate::tr!(
                            "signature, review, what hosts see",
                            "signature, revue, ce que voient les hôtes"
                        ),
                    ),
                    (
                        "portaki reports --open",
                        &crate::tr!(
                            "what hosts and the runtime report",
                            "ce que remontent les hôtes et le runtime"
                        ),
                    ),
                ]);
            }
            ui::advice(crate::tr!(
                "publications are immutable — shipping a change means a new version, never a \
                 re-push of this one",
                "une publication est immuable — livrer un changement, c'est une nouvelle version, \
                 jamais une nouvelle poussée de celle-ci"
            ));
            ui::blank();
            Ok(())
        }
        Outcome::AlreadyPublished => {
            note("state", "already-published");
            // Replaying a publication is not an operator mistake: it is the normal case of a CI
            // that was restarted. The catalogue already carries this version, so there is nothing
            // to do.
            announcing.skip(format!(
                "already in the registry ({} {})",
                coords.id, coords.version
            ));
            ui::advice("nothing to do — a replayed job lands here, and that is fine");
            ui::blank();
            Ok(())
        }
        Outcome::Unauthorized | Outcome::Ignored(_) => {
            announcing.abandon();
            anyhow::bail!(crate::tr!(
                "the registry refused the token — run portaki login, or replay the job. Nothing \
                 is published until the announcement passes: replaying pushes and signs again",
                "le registre a refusé le jeton — lancez portaki login, ou rejouez le job. Rien \
                 n'est publié tant que l'annonce ne passe pas : rejouer pousse et signe à nouveau"
            ))
        }
        Outcome::Refused {
            status,
            code,
            message,
        } => {
            announcing.abandon();
            anyhow::bail!(crate::tr!(
                "the registry refused the publication ({status} {code}): {message}. Nothing is \
                 published: fix it and replay",
                "le registre a refusé la publication ({status} {code}) : {message}. Rien n'est \
                 publié : corrigez, puis rejouez"
            ))
        }
    }
}

/// What authorises the publication — and the two ways of obtaining it.
enum Credential {
    /// Obtained in exchange for the job's OIDC token. No secret is stored anywhere.
    Ci(String),
    /// A person's token, from the keychain or from the environment.
    Person(String),
}

/// Chooses the authorisation path.
///
/// A token set explicitly wins: a mechanism that switches itself on must not silence a variable
/// somebody wrote on purpose. Otherwise, under GitHub Actions, OIDC — and if the workflow did not
/// ask for it, we say so rather than demanding a `portaki login` that is nowhere to be found on a
/// runner.
async fn credential(base: &str, module_id: &str, channel: &str) -> Result<Credential> {
    auth::ensure_transport(base)?;
    if let Some(token) = auth::explicit_token() {
        return Ok(Credential::Person(token));
    }
    if oidc::available() {
        let audience = oidc::audience(base);
        let oidc_token = oidc::request_token(&audience).await?;
        return Ok(Credential::Ci(
            oidc::exchange(base, module_id, channel, &oidc_token).await?,
        ));
    }
    if oidc::inside_github_actions() {
        anyhow::bail!(crate::tr!(
            "no OIDC token available: add `permissions: id-token: write` to the job. The token \
             replaces a publication secret — there is no other one to set",
            "aucun jeton OIDC disponible : ajoutez `permissions: id-token: write` au job. \
             Le jeton remplace un secret de publication — il n'y en a pas d'autre à poser"
        ));
    }
    auth::access_token(base)
        .map(Credential::Person)
        .context(crate::tr!(
            "portaki login required to release",
            "portaki login est nécessaire pour publier"
        ))
}

#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Published,
    /// In the registry but invisible to hosts for as long as its release notes are incomplete:
    /// what is missing, in readable form, and the console page where to complete it.
    Draft {
        missing: Vec<String>,
        url: Option<String>,
    },
    /// Accepted with no effect: the listing is managed in the console, and the registry kept its
    /// own.
    Ignored(String),
    /// This version is already in the catalogue — publications are immutable (ADR-0005).
    AlreadyPublished,
    Unauthorized,
    Refused {
        status: u16,
        code: String,
        message: String,
    },
}

async fn post_publication(base: &str, body: &serde_json::Value, token: &str) -> Result<Outcome> {
    let response = crate::http::client()
        .post(format!(
            "{}/registry/v1/publications",
            base.trim_end_matches('/')
        ))
        .bearer_auth(token)
        .json(body)
        .send()
        .await
        .context("announce the publication to the registry")?;

    Ok(classify(
        response.status().as_u16(),
        &response.text().await.unwrap_or_default(),
    ))
}

/// The registry's refusal body carries a stable `code` — that is what tells "already published"
/// apart from a real failure, and it beats guessing from the status alone: a 409 covers a replayed
/// version just as well as a digest already ingested under another name.
fn classify(status: u16, body: &str) -> Outcome {
    let parsed: serde_json::Value = serde_json::from_str(body).unwrap_or(serde_json::Value::Null);
    if (200..300).contains(&status) {
        return accepted(&parsed);
    }
    if status == 401 {
        return Outcome::Unauthorized;
    }
    let code = parsed
        .get("code")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .to_string();
    if code == "version_already_published" || code == "digest_already_published" {
        return Outcome::AlreadyPublished;
    }
    Outcome::Refused {
        status,
        message: parsed
            .get("message")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(body)
            .to_string(),
        code: if code.is_empty() {
            "unknown".to_string()
        } else {
            code
        },
    }
}

/// A 2xx does not always mean "done": a listing may be ignored, and a version may be left waiting
/// for its notes.
fn accepted(body: &serde_json::Value) -> Outcome {
    let text = |key: &str| body.get(key).and_then(serde_json::Value::as_str);
    if body.get("ignored").and_then(serde_json::Value::as_bool) == Some(true) {
        return Outcome::Ignored(text("message").unwrap_or("ignored").to_string());
    }
    if text("releaseState") != Some("draft") {
        return Outcome::Published;
    }
    let missing = body
        .get("missing")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .map(|item| {
            let field = |key: &str| item.get(key).and_then(serde_json::Value::as_str);
            let lang = field("lang").unwrap_or("?");
            let file = crate::changelog::file_for(lang);
            match (field("kind"), field("permission")) {
                (Some("permissionReason"), Some(permission)) => {
                    format!("justification de la permission {permission} ({lang})")
                }
                (Some("changelogRewrite"), _) => format!(
                    "changelog ({lang}) à réécrire pour les hôtes, pas en message de commit — \
                     dans {file} ou --notes {lang}:…"
                ),
                _ => format!("changelog ({lang}) — ajoutez {file} ou --notes {lang}:…"),
            }
        })
        .collect();
    Outcome::Draft {
        missing,
        url: text("completeUrl").map(str::to_string),
    }
}

fn assert_publish_version_matches_env(module_root: &Path, artifact_dir: &Path) -> Result<()> {
    let expected = std::env::var("PORTAKI_PUBLISH_VERSION").ok();
    assert_publish_version_matches(module_root, artifact_dir, expected.as_deref())
}

/// The comparison, without reading the environment: the tests run in parallel in the same
/// process, and two tests that set and then unset the same variable tread on each other.
fn assert_publish_version_matches(
    module_root: &Path,
    artifact_dir: &Path,
    expected: Option<&str>,
) -> Result<()> {
    let Some(expected) = expected.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(());
    };
    let coords = oci::pack::read_module_coordinates(module_root, artifact_dir)?;
    if coords.version == expected {
        return Ok(());
    }
    anyhow::bail!(
        "publish-manifest version {} does not match PORTAKI_PUBLISH_VERSION={} — \
         align Cargo.toml with the git tag and rebuild (portaki build --release)",
        coords.version,
        expected
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn notes_default_to_the_listings_first_language_else_french() {
        let dir = tempdir().unwrap();
        assert_eq!(notes_lang(&publish_args(&[]), dir.path()), "fr");
        fs::write(dir.path().join(LISTING), r#"{"publishedLangs":["de"]}"#).unwrap();
        assert_eq!(notes_lang(&publish_args(&[]), dir.path()), "de");
        assert_eq!(
            notes_lang(&publish_args(&["--notes-lang", "en"]), dir.path()),
            "en"
        );
    }

    /// A draft refused by `--require-available` still lets the listing go out.
    #[test]
    fn a_refused_draft_still_sends_the_listing() {
        let draft: Result<Landed> = Err(DraftRefused {
            id: "nuki".to_string(),
            version: "1.0.0".to_string(),
        }
        .into());

        assert_eq!(listing_plan(&draft), ListingPlan::Send("nuki"));
        assert!(settle(draft).is_err());
    }

    /// A module whose tests fail: no build, no packing, no push — whatever the flags.
    fn module_with_failing_tests() -> tempfile::TempDir {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"failing-publish\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n",
        )
        .unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/lib.rs"), "//! Fixture.\n").unwrap();
        fs::create_dir_all(dir.path().join("tests")).unwrap();
        fs::write(
            dir.path().join("tests/conformance.rs"),
            // Formatted the way rustfmt wants it: the gate runs `cargo fmt --check` before tests.
            "mod portaki_conformance {\n    #[test]\n    fn surfaces() {\n        panic!(\"home.card panicked\")\n    }\n}\n",
        )
        .unwrap();
        dir
    }

    fn refused(code: &str) -> Result<()> {
        let body = format!(r#"{{"code":"{code}","message":"x"}}"#);
        Err(anyhow::Error::from(oidc::Refused::from_response(
            403, &body,
        )))
    }

    /// A refusal does not stop the ones that follow; the list of unlinked modules and the exit
    /// code come from the run as a whole.
    #[test]
    fn every_module_gets_a_result_and_one_failure_fails_the_run() {
        let outcomes = vec![
            ("access-guide".to_string(), refused("module_not_linked")),
            ("checklist".to_string(), Ok(())),
            ("nuki".to_string(), refused("module_not_linked")),
            ("rules".to_string(), refused("workflow_not_allowed")),
        ];

        assert_eq!(unlinked(&outcomes), vec!["access-guide", "nuki"]);
        let error = conclude(outcomes).unwrap_err().to_string();
        assert_eq!(error, "3 of 4 modules failed");
    }

    #[test]
    fn a_run_where_everything_passed_succeeds() {
        let outcomes = vec![("a".to_string(), Ok(())), ("b".to_string(), Ok(()))];

        assert!(conclude(outcomes).is_ok());
    }

    /// A single module that fails for another reason keeps its own original error.
    #[test]
    fn a_single_module_keeps_its_own_error() {
        let error = conclude(vec![("nuki".to_string(), refused("environment_required"))])
            .unwrap_err()
            .to_string();

        assert!(error.contains("environment_required"), "{error}");
        let error = conclude(vec![("nuki".to_string(), refused("module_not_linked"))])
            .unwrap_err()
            .to_string();
        assert_eq!(error, "nuki was not published");
    }

    #[test]
    fn a_module_without_a_listing_reads_none() {
        let dir = tempdir().unwrap();

        assert!(read_listing(dir.path()).unwrap().is_none());
    }

    #[test]
    fn a_listing_is_read_as_is() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join(LISTING),
            r#"{"category":"access","tagline":"Open the door","publishedLangs":["fr"]}"#,
        )
        .unwrap();

        let listing = read_listing(dir.path()).unwrap().unwrap();
        assert_eq!(listing["tagline"], "Open the door");
    }

    /// A broken listing stops everything before the push, not after.
    #[test]
    fn a_broken_listing_fails_before_publishing() {
        let dir = tempdir().unwrap();
        for raw in [r#"{"category":"#, r#"["access"]"#] {
            fs::write(dir.path().join(LISTING), raw).unwrap();

            let error = format!("{:#}", read_listing(dir.path()).unwrap_err());
            assert!(error.contains(LISTING), "{raw}: {error}");
        }
    }

    #[test]
    fn the_listing_goes_out_once_the_version_is_in_the_registry() {
        let published = Ok(Landed::InRegistry("nuki".to_string()));
        let already: Result<Landed> = Err(AlreadyInRegistry {
            id: "nuki".to_string(),
            version: "1.0.0".to_string(),
            digest: "sha256:aaa".to_string(),
        }
        .into());

        assert_eq!(listing_plan(&published), ListingPlan::Send("nuki"));
        assert_eq!(listing_plan(&already), ListingPlan::Send("nuki"));
        assert_eq!(listing_plan(&Ok(Landed::DryRun)), ListingPlan::WouldSend);
        assert_eq!(
            listing_plan(&refused("module_not_linked").map(|()| Landed::DryRun)),
            ListingPlan::Skip
        );
    }

    /// Replayed on a published version — to push a corrected listing — the run succeeds.
    #[test]
    fn an_already_published_version_settles_as_in_the_registry() {
        let already: Result<Landed> = Err(AlreadyInRegistry {
            id: "nuki".to_string(),
            version: "1.0.0".to_string(),
            digest: "sha256:aaa".to_string(),
        }
        .into());

        assert_eq!(
            settle(already).unwrap(),
            Landed::InRegistry("nuki".to_string())
        );
        assert!(settle(refused("module_not_linked").map(|()| Landed::DryRun)).is_err());
    }

    #[test]
    fn a_refused_listing_carries_the_registry_reasons() {
        let outcome = classify(
            400,
            r#"{"code":"listing_invalid","message":"tagline too long; category unknown"}"#,
        );

        let error = listing_verdict("nuki", outcome).unwrap_err().to_string();
        assert!(error.contains("listing_invalid"), "{error}");
        assert!(error.contains("category unknown"), "{error}");
        assert!(listing_verdict("nuki", classify(204, "")).is_ok());
    }

    /// Published but the listing refused: a run failure like any other, with its reason.
    #[test]
    fn a_refused_listing_fails_the_run() {
        let listing = listing_verdict(
            "nuki",
            classify(403, r#"{"code":"module_name_not_owned","message":"x"}"#),
        );
        let outcomes = vec![
            ("checklist".to_string(), Ok(())),
            ("nuki".to_string(), listing),
        ];

        assert_eq!(
            conclude(outcomes).unwrap_err().to_string(),
            "1 of 2 modules failed"
        );
        let alone = listing_verdict(
            "nuki",
            classify(403, r#"{"code":"module_name_not_owned","message":"x"}"#),
        );
        let error = conclude(vec![("nuki".to_string(), alone)])
            .unwrap_err()
            .to_string();
        assert!(error.contains("module_name_not_owned"), "{error}");
    }

    fn publish_args(flags: &[&str]) -> ReleaseArgs {
        let mut argv = vec!["release"];
        argv.extend_from_slice(flags);
        ReleaseArgs::try_parse_from(argv).unwrap()
    }

    /// `ci build` is what the job without rights runs, `--dry-run` what a pull request runs:
    /// every path that builds stops at failing tests, before any packing.
    #[tokio::test]
    async fn failing_tests_stop_every_publication_path() {
        for (flags, mode) in [
            (&["--dry-run"][..], Mode::Local),
            (&["--no-sign"][..], Mode::Local),
            (&[][..], Mode::CiBuild),
        ] {
            let module = module_with_failing_tests();

            let error = run_in(module.path(), &publish_args(flags), &mode)
                .await
                .unwrap_err();

            let chain = format!("{error:#}");
            assert!(chain.contains("tests before publish"), "{flags:?}: {chain}");
            assert!(chain.contains("tests fail"), "{flags:?}: {chain}");
            assert!(
                !module.path().join("target/portaki").exists(),
                "{flags:?}: nothing may be built or packed once the tests fail"
            );
        }
    }

    /// `module_with_failing_tests`, plus the artifact an earlier job built — named `artifact_id`.
    fn prebuilt_module(artifact_id: &str) -> tempfile::TempDir {
        let module = module_with_failing_tests();
        let artifact = module.path().join("target/portaki");
        fs::create_dir_all(&artifact).unwrap();
        fs::write(
            artifact.join("publish-manifest.json"),
            format!(r#"{{"id":"{artifact_id}","version":"0.1.0","sdkVersion":"8.7.0"}}"#),
        )
        .unwrap();
        let wasm = module.path().join("target/wasm32-unknown-unknown/release");
        fs::create_dir_all(&wasm).unwrap();
        fs::write(wasm.join(format!("{artifact_id}.wasm")), b"\0asm").unwrap();
        fs::write(
            module.path().join("Cargo.lock"),
            "[[package]]\nname = \"portaki-sdk\"\nversion = \"8.7.0\"\n",
        )
        .unwrap();
        module
    }

    fn ci_release() -> Mode {
        Mode::CiRelease { audit: None }
    }

    /// The publishing job runs nothing of the module: the tests would fail here, and are not run.
    #[tokio::test]
    async fn prebuilt_runs_no_module_code() {
        let module = prebuilt_module("failing-publish");

        let landed = release(
            module.path(),
            &publish_args(&["--dry-run"]),
            &ci_release(),
            "http://127.0.0.1:1",
        )
        .await;

        assert_eq!(landed.unwrap(), Landed::DryRun);
    }

    /// An artifact built by module code does not pick the name it is pushed under.
    #[tokio::test]
    async fn prebuilt_refuses_an_artifact_named_after_another_module() {
        let module = prebuilt_module("nuki");

        let error = release(
            module.path(),
            &publish_args(&["--dry-run"]),
            &ci_release(),
            "http://127.0.0.1:1",
        )
        .await
        .unwrap_err();

        assert!(
            format!("{error:#}").contains("refusing to publish"),
            "{error:#}"
        );
    }

    /// The SDK an artifact declares is the one Cargo.lock resolves — read without cargo.
    #[tokio::test]
    async fn ci_release_refuses_an_sdk_the_lock_does_not_resolve() {
        let module = prebuilt_module("failing-publish");
        fs::write(
            module.path().join("Cargo.lock"),
            "[[package]]\nname = \"portaki-sdk\"\nversion = \"8.9.0\"\n",
        )
        .unwrap();

        let error = release(
            module.path(),
            &publish_args(&["--dry-run"]),
            &ci_release(),
            "http://127.0.0.1:1",
        )
        .await
        .unwrap_err();

        let chain = format!("{error:#}");
        assert!(chain.contains("declares portaki-sdk 8.7.0"), "{chain}");
        assert!(chain.contains("resolves 8.9.0"), "{chain}");
        fs::remove_file(module.path().join("Cargo.lock")).unwrap();
        let error = release(
            module.path(),
            &publish_args(&["--dry-run"]),
            &ci_release(),
            "http://127.0.0.1:1",
        )
        .await
        .unwrap_err();
        assert!(format!("{error:#}").contains("no Cargo.lock"), "{error:#}");
    }

    /// The registry's answer, handed as is to the OCI client — or its refusal, with its code.
    #[test]
    fn a_push_grant_is_read_and_a_refusal_keeps_its_code() {
        let grant = grant_from(
            201,
            r#"{"registry":"oci-staging.portaki.app","repository":"modules/nuki",
                "reference":"oci-staging.portaki.app/modules/nuki:1.4.0","username":"portaki-push",
                "password":"pk_push_x","moduleId":"nuki","version":"1.4.0","channel":"stable",
                "expiresAt":"2026-09-27T12:00:00Z"}"#,
        )
        .unwrap();
        assert_eq!(grant.registry, "oci-staging.portaki.app");
        assert_eq!(grant.password, "pk_push_x");

        let refused = grant_from(
            403,
            r#"{"code":"official_requires_ci","message":"nuki est un module officiel"}"#,
        )
        .unwrap_err()
        .to_string();
        assert!(refused.contains("official_requires_ci"), "{refused}");
        assert!(grant_from(401, "").is_err());
    }

    #[test]
    fn notes_repeat_one_line_each() {
        let args = publish_args(&["--notes", "Keypad code", "--notes", "Faster sync"]);

        assert_eq!(args.notes, vec!["Keypad code", "Faster sync"]);
        assert_eq!(args.notes_lang, None);
    }

    #[test]
    fn one_layer_is_not_layers() {
        assert_eq!(plural(1, "layer"), "1 layer");
        assert_eq!(plural(5, "layer"), "5 layers");
        assert_eq!(plural(0, "layer"), "0 layers");
    }

    /// The catalogue returns every version: ours is the one to find in it, not whichever comes
    /// first — otherwise a republication would be refused in the name of another version.
    #[test]
    fn the_catalogue_is_searched_for_our_own_version() {
        let published: Vec<PublishedVersion> = serde_json::from_value(serde_json::json!([
            { "digest": "sha256:aaa", "version": "0.3.1" },
            { "digest": "sha256:bbb", "version": "0.3.2" }
        ]))
        .unwrap();

        assert_eq!(
            digest_of(&published, "0.3.2").as_deref(),
            Some("sha256:bbb")
        );
        assert!(digest_of(&published, "0.4.0").is_none());
        assert!(digest_of(&[], "0.3.2").is_none());
    }

    #[test]
    fn an_ignored_listing_is_not_sent() {
        assert_eq!(
            classify(
                200,
                r#"{"ignored":true,"message":"listing.json ignoré : la fiche est gérée dans la console"}"#
            ),
            Outcome::Ignored("listing.json ignoré : la fiche est gérée dans la console".into())
        );
        assert_eq!(classify(200, "{}"), Outcome::Published);
        assert_eq!(classify(204, ""), Outcome::Published);
    }

    #[test]
    fn a_draft_says_what_is_missing_and_where() {
        let body = r#"{"releaseState":"draft","completeUrl":"https://developer.portaki.app/nuki/release",
            "missing":[{"kind":"changelog","lang":"fr"},
                       {"kind":"permissionReason","permission":"email","lang":"en"}]}"#;

        assert_eq!(
            classify(201, body),
            Outcome::Draft {
                missing: vec![
                    "changelog (fr) — ajoutez CHANGELOG.fr.md ou --notes fr:…".into(),
                    "justification de la permission email (en)".into()
                ],
                url: Some("https://developer.portaki.app/nuki/release".into()),
            }
        );
        assert_eq!(
            classify(201, r#"{"releaseState":"available"}"#),
            Outcome::Published
        );
    }

    #[test]
    fn release_flags_are_parsed() {
        let args = publish_args(&[
            "--notes",
            "fr:Code clavier",
            "--permission-reason",
            "email=Pour le code",
            "--host-action",
            "fr:Reconnecter",
        ]);

        assert_eq!(args.permission_reasons, vec!["email=Pour le code"]);
        assert_eq!(args.host_action, vec!["fr:Reconnecter"]);
        assert!(!args.host_action_required);
    }

    #[test]
    fn a_replayed_publication_is_not_an_error() {
        let outcome = classify(
            409,
            r#"{"code":"version_already_published","message":"1.4.0"}"#,
        );

        assert_eq!(outcome, Outcome::AlreadyPublished);
    }

    /// A 409 is not enough: the same status also covers a refusal there is nothing to conclude
    /// from.
    #[test]
    fn another_conflict_is_still_a_refusal() {
        let outcome = classify(409, r#"{"code":"something_else","message":"nope"}"#);

        assert!(matches!(outcome, Outcome::Refused { .. }));
    }

    #[test]
    fn a_refusal_keeps_the_registry_code_so_a_ci_knows_why() {
        let outcome = classify(403, r#"{"code":"module_name_not_owned","message":"nuki"}"#);

        match outcome {
            Outcome::Refused { status, code, .. } => {
                assert_eq!(status, 403);
                assert_eq!(code, "module_name_not_owned");
            }
            other => panic!("attendu un refus, obtenu {other:?}"),
        }
    }

    /// An empty or unreadable body must not let a failure pass for a success.
    #[test]
    fn an_unreadable_refusal_is_still_a_refusal() {
        let outcome = classify(500, "<html>oops</html>");

        match outcome {
            Outcome::Refused { code, .. } => assert_eq!(code, "unknown"),
            other => panic!("attendu un refus, obtenu {other:?}"),
        }
    }

    #[test]
    fn assert_publish_version_matches_env_accepts_matching_version() {
        let root = tempdir().unwrap();
        let artifact = root.path().join("target/portaki");
        fs::create_dir_all(&artifact).unwrap();
        fs::write(
            artifact.join(oci::pack::PUBLISH_MANIFEST),
            r#"{"id":"weather","version":"0.2.1"}"#,
        )
        .unwrap();
        assert_publish_version_matches(root.path(), &artifact, Some("0.2.1")).unwrap();
    }

    #[test]
    fn assert_publish_version_matches_env_rejects_mismatch() {
        let root = tempdir().unwrap();
        let artifact = root.path().join("target/portaki");
        fs::create_dir_all(&artifact).unwrap();
        fs::write(
            artifact.join(oci::pack::PUBLISH_MANIFEST),
            r#"{"id":"weather","version":"0.1.0"}"#,
        )
        .unwrap();
        let err =
            assert_publish_version_matches(root.path(), &artifact, Some("0.2.1")).unwrap_err();
        assert!(err.to_string().contains("0.1.0"));
        assert!(err.to_string().contains("0.2.1"));
    }
}
