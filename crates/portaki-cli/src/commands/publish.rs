//! `portaki publish` — OCI push via `oci-distribution` (ORAS-compatible layout).
//!
//! Runs the module's tests first — `cargo test` on the host, the conformance battery of
//! `portaki_test_utils::conformance!()` required among them — and refuses to go further when they
//! fail. `--dry-run` and `--skip-build` run them too; only `--announce-only`, which compiles and
//! pushes nothing, does not.
//!
//! Then runs `portaki build --release` (unless `--skip-build`) so the OCI catalog layer
//! comes from `target/portaki/publish-manifest.json`, not a hand-edited repo file at publish time.
//!
//! Authenticates with `GITHUB_TOKEN` / `GHCR_TOKEN` or Docker `~/.docker/config.json`.
//!
//! L'annonce au registre, elle, n'a plus besoin d'un secret stocké : dans un job GitHub Actions
//! avec `id-token: write`, la CLI demande le jeton OIDC du job et l'échange contre un credential
//! de publication à usage unique. Hors CI, le jeton de `portaki login` fait le travail.
//!
//! Set `PORTAKI_PUBLISH_VERSION` (e.g. from CI git tag `*-vX.Y.Z`) to fail fast if `publish-manifest.json`
//! version does not match.
//!
//! Après la poussée OCI, la publication est **annoncée au registre Portaki**. Sans cette annonce
//! l'artefact existe sur GHCR mais n'entre dans aucun catalogue : c'est ce qui manquait pour que
//! l'orchestrator puisse lire son catalogue depuis le registre plutôt que depuis GHCR.
//!
//! Puis la fiche publique `listing.json`, si le module en a une, part au registre — y compris
//! quand la version y était déjà.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Parser;

use crate::commands::build::{self, BuildArgs};
use crate::commands::{link, test};
use crate::{auth, oci, oidc, ui, workspace};

#[derive(Debug, Clone, Parser)]
/// Arguments for `portaki publish`.
pub struct PublishArgs {
    /// OCI registry prefix. Defaults to ghcr.io/portakiapp for official modules; required
    /// for any other author.
    #[arg(long)]
    pub registry: Option<String>,
    /// Run the tests and validate packaging without pushing.
    #[arg(long)]
    pub dry_run: bool,
    /// Artifact directory (defaults to `target/portaki`).
    #[arg(long)]
    pub artifact_dir: Option<PathBuf>,
    /// Skip the implicit `portaki build --release` (not recommended). The tests still run.
    #[arg(long)]
    pub skip_build: bool,
    /// Release channel at the Portaki registry.
    #[arg(long, default_value = "stable")]
    pub channel: String,
    /// Base URL of the platform. Defaults to PORTAKI_API_URL, then production.
    #[arg(long)]
    pub url: Option<String>,
    /// Push to GHCR without announcing it — the artifact then enters no catalogue.
    #[arg(long)]
    pub no_announce: bool,
    /// Announce a version already on GHCR, compiling and pushing nothing.
    #[arg(long, conflicts_with_all = ["no_announce", "dry_run", "skip_build"])]
    pub announce_only: bool,
    /// In a repository holding several modules, the one to publish.
    #[arg(long, conflicts_with = "all")]
    pub module: Option<String>,
    /// Publish every module of the repository, each on its own.
    #[arg(long)]
    pub all: bool,
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

/// The namespace Portaki publishes its own modules under.
const OFFICIAL_REGISTRY: &str = "ghcr.io/portakiapp";

/// Where this module's artifact belongs.
///
/// The old default pushed everything to the Portaki namespace. For a module whose manifest
/// names someone else as its author that is the wrong place, and a default nobody notices is a
/// default that gets noticed after the push.
fn resolve_registry(flag: Option<&str>, module_root: &Path) -> Result<String> {
    if let Some(registry) = flag {
        return Ok(registry.to_string());
    }
    if author_type(module_root).as_deref() == Some("official") {
        return Ok(OFFICIAL_REGISTRY.to_string());
    }
    anyhow::bail!(
        "--registry is required: {OFFICIAL_REGISTRY} is the Portaki namespace, and \
         portaki.module.json does not declare an official module — pass your own, \
         e.g. --registry ghcr.io/<owner>"
    )
}

/// `author.type` as the catalogue manifest declares it, when it can be read at all.
fn author_type(module_root: &Path) -> Option<String> {
    let raw = std::fs::read_to_string(module_root.join("portaki.module.json")).ok()?;
    let manifest: serde_json::Value = serde_json::from_str(&raw).ok()?;
    manifest
        .get("author")?
        .get("type")?
        .as_str()
        .map(str::to_string)
}

/// Runs `portaki publish`.
pub async fn run(args: PublishArgs) -> Result<()> {
    ui::header(
        "portaki publish",
        "Push the OCI artifact, then announce it so a catalogue can carry it.",
    );

    // Un module après l'autre, chacun avec son jeton et son digest : un refus n'arrête pas les
    // suivants, et le code de sortie dit s'il y en a eu un.
    let chosen = workspace::resolve(args.module.as_deref(), Some(args.all))?;
    let mut outcomes = Vec::with_capacity(chosen.len());
    for member in &chosen {
        if chosen.len() > 1 {
            ui::rule(&member.id);
        }
        workspace::enter(member)?;
        let outcome = run_in(&member.root, args.clone()).await;
        outcomes.push((member.id.clone(), outcome));
    }
    conclude(outcomes)
}

/// Le refus `module_not_linked` de l'échange OIDC, s'il est dans la chaîne.
fn not_linked(failure: &anyhow::Error) -> Option<&oidc::Refused> {
    failure
        .chain()
        .find_map(|cause| cause.downcast_ref::<oidc::Refused>())
        .filter(|refused| refused.code == "module_not_linked")
}

/// Les modules refusés faute de liaison, dans l'ordre du run.
///
/// Le CLI ne sait pas lister les liaisons — l'API qui le dit veut un jeton de développeur, et
/// une CI n'a que son jeton OIDC. Ce sont donc les refus de ce run qui font la liste.
fn unlinked(outcomes: &[(String, Result<()>)]) -> Vec<String> {
    outcomes
        .iter()
        .filter(|(_, outcome)| outcome.as_ref().err().and_then(not_linked).is_some())
        .map(|(id, _)| id.clone())
        .collect()
}

/// Un résultat par module, le lien pour lier d'un coup ceux qui ne le sont pas, et un échec
/// si un seul module n'est pas passé.
fn conclude(outcomes: Vec<(String, Result<()>)>) -> Result<()> {
    let unlinked = unlinked(&outcomes);
    // La page Dépôt vient du registre, dans le refus du premier module non lié.
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
                ui::success(format!("{id} published"));
            }
            continue;
        };
        failed += 1;
        if let Some(refused) = not_linked(&failure) {
            status = refused.status;
            if total > 1 {
                ui::failure(format!("{id} — not linked to any repository"));
            }
        } else if total == 1 {
            // Un module seul : l'échec remonte tel quel, comme avant.
            return Err(failure);
        } else {
            ui::failure(format!("{id} — {failure:#}"));
        }
    }

    if let Some(first) = unlinked.first() {
        ui::blank();
        ui::failure(format!(
            "{status} module_not_linked — « {first} » n'est lié à aucun dépôt"
        ));
        ui::detail(format!(
            "Modules non liés dans ce dépôt : {}",
            unlinked.join(", ")
        ));
        if let Some(page) = &page {
            ui::detail(format!(
                "→ Liez-les en une fois : {}",
                link::with_also(page, &unlinked[1..])
            ));
        }
        ui::blank();
    }

    match failed {
        0 => Ok(()),
        _ if total == 1 => anyhow::bail!("{} was not published", unlinked.join(", ")),
        _ => anyhow::bail!("{failed} of {total} modules failed"),
    }
}

/// La fiche publique du module, versionnée à côté de `portaki.module.json`.
const LISTING: &str = "listing.json";

/// La fiche du module, lue et vérifiée avant toute publication : une fiche cassée découverte
/// après la poussée laisserait un artefact publié et une vitrine en retard.
///
/// Le contenu part tel quel — c'est le registre qui en valide les champs.
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

/// Jusqu'où la publication est allée.
#[derive(Debug, PartialEq, Eq)]
enum Landed {
    DryRun,
    /// Poussé sur GHCR sans annonce (`--no-announce`) : le module peut n'être dans aucun catalogue.
    Unannounced,
    /// La version est au registre — annoncée à l'instant, ou déjà là.
    InRegistry(String),
}

/// La version est déjà au registre : la publication s'arrête avant de pousser (ADR-0005).
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
            "{} {} is already in the registry ({}) — publications are immutable, so \
             pushing again would leave the OCI tag pointing at something the catalogue does not \
             reference. Bump the version, or replay the announcement with \
             portaki publish --announce-only",
            self.id, self.version, self.digest
        )
    }
}

impl std::error::Error for AlreadyInRegistry {}

/// Ce qu'on fait de la fiche, selon où la publication s'est arrêtée.
#[derive(Debug, PartialEq, Eq)]
enum ListingPlan<'a> {
    Send(&'a str),
    WouldSend,
    Skip,
}

/// La fiche part dès que la version est au registre — y compris déjà publiée, sans quoi une
/// fiche corrigée attendrait la release suivante.
fn listing_plan(landed: &Result<Landed>) -> ListingPlan<'_> {
    match landed {
        Ok(Landed::InRegistry(id)) => ListingPlan::Send(id),
        Ok(Landed::DryRun) => ListingPlan::WouldSend,
        Ok(Landed::Unannounced) => ListingPlan::Skip,
        Err(failure) => match failure.downcast_ref::<AlreadyInRegistry>() {
            Some(already) => ListingPlan::Send(&already.id),
            None => ListingPlan::Skip,
        },
    }
}

/// Une version déjà au registre n'est pas un échec : rien n'est poussé, et la suite — la fiche —
/// part quand même.
///
/// C'était une erreur, et c'est ce qui rendait rouge un run relancé seulement pour pousser une
/// fiche corrigée. L'action de release attend d'ailleurs ce cas comme une issue normale
/// (`already-published`) : elle lit « already in the registry » sur la sortie standard, d'où
/// l'avertissement plutôt qu'une erreur.
fn settle(landed: Result<Landed>) -> Result<Landed> {
    match landed {
        Err(failure) => match failure.downcast_ref::<AlreadyInRegistry>() {
            Some(already) => {
                ui::warn(already);
                Ok(Landed::InRegistry(already.id.clone()))
            }
            None => Err(failure),
        },
        landed => landed,
    }
}

/// `portaki publish` for the module in `module_root`, then its public listing.
async fn run_in(module_root: &Path, args: PublishArgs) -> Result<()> {
    let Some(listing) = read_listing(module_root)? else {
        return settle(release(module_root, &args).await).map(|_| ());
    };
    let landed = settle(release(module_root, &args).await);
    let sent = match listing_plan(&landed) {
        ListingPlan::Send(id) => send_listing(&args, id, &listing).await,
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

/// Envoie la fiche au registre. Elle remplace celle éditée dans le dashboard : le dépôt fait foi.
///
/// Un credential de CI est à usage unique et l'annonce a consommé le sien : on en redemande un.
async fn send_listing(
    args: &PublishArgs,
    module_id: &str,
    listing: &serde_json::Value,
) -> Result<()> {
    let base = auth::api_base_url(args.url.as_deref());
    let outcome = match credential(&base, module_id, &args.channel).await? {
        Credential::Ci(token) => put_listing(&base, module_id, listing, &token).await?,
        Credential::Person(token) => {
            let first = put_listing(&base, module_id, listing, &token).await?;
            if first == Outcome::Unauthorized {
                put_listing(
                    &base,
                    module_id,
                    listing,
                    &auth::refresh(&base, &token).await?,
                )
                .await?
            } else {
                first
            }
        }
    };
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

/// Une fiche refusée n'annule pas la publication : elle fait échouer le run, avec le motif.
fn listing_verdict(module_id: &str, outcome: Outcome) -> Result<()> {
    match outcome {
        Outcome::Published => Ok(()),
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

/// `portaki publish` for the module in `module_root`, up to the announcement.
async fn release(module_root: &Path, args: &PublishArgs) -> Result<Landed> {
    let module_root = module_root.to_path_buf();
    let artifact_dir = args
        .artifact_dir
        .clone()
        .unwrap_or_else(|| module_root.join("target/portaki"));
    let registry = resolve_registry(args.registry.as_deref(), &module_root)?;

    // Reprise d'un catalogue déjà sur GHCR : on lit le digest de la version publiée et on
    // l'annonce. Rien n'est recompilé ni renvoyé, donc aucun droit d'écriture nécessaire — et
    // aucun risque d'écraser un artefact par un build local qui aurait dérivé.
    if args.announce_only {
        let coords = oci::pack::read_source_coordinates(&module_root)?;
        let looking = ui::step("looking up the pushed artifact");
        let pushed = oci::resolve_pushed_artifact(&module_root, &registry).await?;
        looking.done("found the artifact on the registry");
        ui::field("image", &pushed.image_ref);
        ui::field("digest", &pushed.digest);
        announce(args, &coords, &pushed).await?;
        return Ok(Landed::InRegistry(coords.id));
    }

    // Avant tout build : un module dont les tests échouent n'a rien à pousser, et la batterie de
    // conformité est ce que tous les modules doivent à la plateforme. Pas de drapeau pour
    // l'éviter — `--skip-build` saute un artefact qu'un job précédent a produit, et les tests ne
    // sont pas un artefact qu'on se passe.
    test::gate_publish(&module_root).context("tests before publish")?;
    ui::blank();

    if args.skip_build {
        ui::skipped("build skipped (--skip-build)");
    } else {
        build::run(BuildArgs {
            release: true,
            manifest_only: false,
            module: None,
            all: false,
            nested: true,
        })
        .await
        .context("portaki build --release before publish")?;
        ui::blank();
    }

    let packing = ui::step("packing the OCI artifact");
    // The layer list the push would send, assembled here rather than at push time: it is what
    // says the wasm exists. A dry run that skipped it answered a question it had not checked.
    let layers =
        oci::pack::collect_push_layers(&module_root, &artifact_dir).map_err(|failure| {
            packing.abandon();
            failure
        })?;
    assert_publish_version_matches_env(&module_root, &artifact_dir)?;
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

    if args.dry_run {
        ui::success("dry run — nothing was pushed, nothing was announced");
        ui::field("artifact", artifact_dir.display());
        ui::field("registry", &registry);
        ui::advice("drop --dry-run to push these layers and announce the version");
        ui::blank();
        return Ok(Landed::DryRun);
    }

    // Demandé avant de pousser, pas découvert après. Une publication est immuable (ADR-0005) :
    // le second envoi se faisait refuser à l'annonce, mais il avait déjà écrasé le tag OCI —
    // qui ne désignait alors plus l'artefact que le catalogue référence.
    let coords = oci::pack::read_module_coordinates(&module_root, &artifact_dir)?;
    refuse_if_already_published(&auth::api_base_url(args.url.as_deref()), &coords).await?;

    let pushing = ui::step(format!("pushing to {registry}"));
    let pushed = oci::push_artifact(&module_root, &artifact_dir, &registry)
        .await
        .map_err(|failure| {
            pushing.abandon();
            failure
        })
        .context("push OCI artifact — set GITHUB_TOKEN or docker login ghcr.io")?;
    pushing.done(format!("pushed to {registry}"));
    ui::field("manifest", &pushed.manifest_url);

    if args.no_announce {
        ui::warn("skipped the registry announcement — this version is in no catalogue");
        ui::advice("drop --no-announce, or replay with portaki publish --announce-only");
        ui::blank();
        return Ok(Landed::Unannounced);
    }

    announce(args, &coords, &pushed).await?;
    Ok(Landed::InRegistry(coords.id))
}

/// Une version publiée ne se republie pas.
///
/// Le registre le disait déjà, mais à l'annonce — c'est-à-dire après la poussée OCI. Le tag
/// avait donc été réécrit, et ne désignait plus l'artefact dont le catalogue porte le digest :
/// deux sources de vérité en désaccord, sans que rien ne le signale.
///
/// Le catalogue est public : la question ne coûte ni jeton ni droit.
///
/// Injoignable, on continue. Refuser de publier parce qu'une lecture de contrôle échoue
/// bloquerait une livraison pour une raison qui n'en est pas une, et l'annonce refusera de
/// toute façon si la version existe.
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

/// Une version au catalogue, telle que le registre la rend.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PublishedVersion {
    digest: String,
    version: String,
}

/// Le digest publié pour cette version, s'il y en a un.
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

/// La sélection seule, séparée du réseau pour être vérifiable.
fn digest_of(published: &[PublishedVersion], version: &str) -> Option<String> {
    published
        .iter()
        .find(|candidate| candidate.version == version)
        .map(|candidate| candidate.digest.clone())
}

/// Annonce la publication au registre, en renouvelant le jeton une fois sur un 401.
///
/// L'échec ici n'annule pas la poussée OCI — l'artefact est sur GHCR quoi qu'il arrive. Le
/// message dit donc quoi rejouer, plutôt que de laisser croire que rien n'a eu lieu.
async fn announce(
    args: &PublishArgs,
    coords: &oci::pack::ModuleCoordinates,
    pushed: &oci::PushedArtifact,
) -> Result<()> {
    let base = auth::api_base_url(args.url.as_deref());
    let announcing = ui::step(format!("announcing {} to the registry", args.channel));
    let body = serde_json::json!({
        "moduleId": coords.id,
        "version": coords.version,
        "artifactRef": pushed.artifact_ref(),
        "digest": pushed.digest,
        "channel": args.channel,
    });

    let outcome = match credential(&base, &coords.id, &args.channel).await? {
        // Une CI : le credential est à usage unique, un 401 veut dire consommé ou expiré. Le
        // rejouer avec le même n'aurait aucune chance, il faut un nouvel échange.
        Credential::Ci(token) => post_publication(&base, &body, &token).await?,
        Credential::Person(token) => {
            let first = post_publication(&base, &body, &token).await?;
            if first == Outcome::Unauthorized {
                post_publication(&base, &body, &auth::refresh(&base, &token).await?).await?
            } else {
                first
            }
        }
    };

    match outcome {
        Outcome::Published => {
            announcing.done(format!("announced to the registry on {}", args.channel));
            ui::field("module", format!("{} {}", coords.id, coords.version));
            ui::field("channel", &args.channel);
            ui::field("digest", &pushed.digest);
            ui::advice(
                "publications are immutable — shipping a change means a new version, never a \
                 re-push of this one",
            );
            ui::blank();
            Ok(())
        }
        Outcome::AlreadyPublished => {
            // Rejouer une publication n'est pas une erreur d'opérateur : c'est le cas normal
            // d'une CI relancée. Le catalogue porte déjà cette version, il n'y a rien à faire.
            announcing.skip(format!(
                "already in the registry ({} {})",
                coords.id, coords.version
            ));
            ui::advice("nothing to do — a replayed job lands here, and that is fine");
            ui::blank();
            Ok(())
        }
        Outcome::Unauthorized => {
            announcing.abandon();
            anyhow::bail!(
                "the registry refused the token — run portaki login, or replay the job if the \
             publication credential had already been used. \
             The artifact is on GHCR: replay with portaki publish --skip-build"
            )
        }
        Outcome::Refused {
            status,
            code,
            message,
        } => {
            announcing.abandon();
            anyhow::bail!(
                "the registry refused the publication ({status} {code}): {message}. \
                 The artifact is on GHCR: fix and replay with portaki publish --skip-build"
            )
        }
    }
}

/// Ce qui autorise la publication — et les deux façons de l'obtenir.
enum Credential {
    /// Obtenu contre le jeton OIDC du job. Aucun secret n'est stocké nulle part.
    Ci(String),
    /// Le jeton d'une personne, depuis le trousseau ou l'environnement.
    Person(String),
}

/// Choisit le chemin d'autorisation.
///
/// Un jeton posé explicitement gagne : un mécanisme qui s'active tout seul ne doit pas rendre
/// muette une variable qu'on a écrite exprès. Sinon, chez GitHub Actions, l'OIDC — et si le
/// workflow ne l'a pas demandé, on le dit plutôt que de réclamer un `portaki login` introuvable
/// sur un runner.
async fn credential(base: &str, module_id: &str, channel: &str) -> Result<Credential> {
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
        anyhow::bail!(
            "aucun jeton OIDC disponible : ajoute `permissions: id-token: write` au job. \
             Le jeton est ce qui remplace un secret de publication — il n'y en a pas d'autre à poser"
        );
    }
    auth::access_token()
        .map(Credential::Person)
        .context("portaki login required to announce a publication — or pass --no-announce to push to GHCR only")
}

#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Published,
    /// Cette version est déjà au catalogue — les publications sont immuables (ADR-0005).
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

/// Le corps de refus du registre porte un `code` stable — c'est lui qui distingue « déjà publié »
/// d'un vrai échec, et il vaut mieux que deviner à partir du seul statut : un 409 recouvre aussi
/// bien une version rejouée qu'un digest déjà ingéré sous un autre nom.
fn classify(status: u16, body: &str) -> Outcome {
    if (200..300).contains(&status) {
        return Outcome::Published;
    }
    if status == 401 {
        return Outcome::Unauthorized;
    }
    let parsed: serde_json::Value = serde_json::from_str(body).unwrap_or(serde_json::Value::Null);
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

fn assert_publish_version_matches_env(module_root: &Path, artifact_dir: &Path) -> Result<()> {
    let expected = std::env::var("PORTAKI_PUBLISH_VERSION").ok();
    assert_publish_version_matches(module_root, artifact_dir, expected.as_deref())
}

/// La comparaison, sans lire l'environnement : les tests tournent en parallèle dans le même
/// processus, et deux tests qui posent puis retirent la même variable se marchent dessus.
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

    /// A module whose tests fail: no build, no packing, no push — whatever the flags.
    fn module_with_failing_tests() -> tempfile::TempDir {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"failing-publish\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n",
        )
        .unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/lib.rs"), "").unwrap();
        fs::create_dir_all(dir.path().join("tests")).unwrap();
        fs::write(
            dir.path().join("tests/conformance.rs"),
            "mod portaki_conformance { #[test] fn surfaces() { panic!(\"home.card panicked\") } }\n",
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

    /// Un refus n'arrête pas les suivants ; la liste des non liés et le code de sortie
    /// viennent du run entier.
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

    /// Un module seul qui échoue pour une autre raison garde son erreur d'origine.
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

    /// Une fiche cassée arrête tout avant la poussée, pas après.
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
        assert_eq!(listing_plan(&Ok(Landed::Unannounced)), ListingPlan::Skip);
        assert_eq!(
            listing_plan(&refused("module_not_linked").map(|()| Landed::DryRun)),
            ListingPlan::Skip
        );
    }

    /// Relancé sur une version publiée — pour pousser une fiche corrigée —, le run réussit.
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

    /// Publié mais fiche refusée : un échec du run comme un autre, avec son motif.
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

    fn publish_args(flags: &[&str]) -> PublishArgs {
        let mut argv = vec!["publish", "--registry", "ghcr.io/someone"];
        argv.extend_from_slice(flags);
        PublishArgs::try_parse_from(argv).unwrap()
    }

    /// The release action publishes with `--skip-build` after its own build, and `--dry-run` is
    /// what a pull request runs: both must stop at failing tests, before any packing.
    #[tokio::test]
    async fn failing_tests_stop_every_publication_path() {
        for flags in [
            &["--dry-run"][..],
            &["--skip-build"][..],
            &["--dry-run", "--skip-build"][..],
            &[][..],
        ] {
            let module = module_with_failing_tests();

            let error = run_in(module.path(), publish_args(flags))
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

    /// Announcing a version already on GHCR compiles nothing: there is nothing to test.
    #[test]
    fn announce_only_does_not_take_the_other_flags() {
        assert!(PublishArgs::try_parse_from(["publish", "--announce-only", "--dry-run"]).is_err());
        assert!(
            PublishArgs::try_parse_from(["publish", "--announce-only", "--skip-build"]).is_err()
        );
    }

    fn module_with_author(author_type: &str) -> tempfile::TempDir {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("portaki.module.json"),
            format!(
                r#"{{"id":"x","version":"0.1.0","author":{{"name":"n","type":"{author_type}"}}}}"#
            ),
        )
        .unwrap();
        dir
    }

    #[test]
    fn one_layer_is_not_layers() {
        assert_eq!(plural(1, "layer"), "1 layer");
        assert_eq!(plural(5, "layer"), "5 layers");
        assert_eq!(plural(0, "layer"), "0 layers");
    }

    #[test]
    fn an_official_module_keeps_the_portaki_namespace() {
        let dir = module_with_author("official");

        assert_eq!(
            resolve_registry(None, dir.path()).unwrap(),
            OFFICIAL_REGISTRY
        );
    }

    /// The default that only gets noticed after the push.
    #[test]
    fn anyone_else_has_to_name_their_own() {
        let dir = module_with_author("community");

        let error = resolve_registry(None, dir.path()).unwrap_err().to_string();
        assert!(error.contains("--registry is required"), "{error}");
        // Given explicitly, the namespace is theirs to choose.
        assert_eq!(
            resolve_registry(Some("ghcr.io/someone"), dir.path()).unwrap(),
            "ghcr.io/someone"
        );
    }

    #[test]
    fn a_manifest_that_cannot_be_read_is_not_treated_as_official() {
        let dir = tempdir().unwrap();

        assert!(resolve_registry(None, dir.path()).is_err());
    }

    /// Le catalogue rend toutes les versions : c'est la nôtre qu'il faut y trouver, pas la
    /// première venue — sans quoi une republication serait refusée au nom d'une autre version.
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
    fn a_replayed_publication_is_not_an_error() {
        let outcome = classify(
            409,
            r#"{"code":"version_already_published","message":"1.4.0"}"#,
        );

        assert_eq!(outcome, Outcome::AlreadyPublished);
    }

    /// Un 409 ne suffit pas : le même statut couvre un refus dont il n'y a rien à conclure.
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

    /// Un corps vide ou illisible ne doit pas faire passer un échec pour un succès.
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
