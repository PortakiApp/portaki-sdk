//! `portaki publish` — OCI push via `oci-distribution` (ORAS-compatible layout).
//!
//! Always runs `portaki build --release` first (unless `--skip-build`) so the OCI catalog layer
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

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Parser;

use crate::commands::build::{self, BuildArgs};
use crate::{auth, oci, oidc, ui};

#[derive(Debug, Parser)]
/// Arguments for `portaki publish`.
pub struct PublishArgs {
    /// OCI registry prefix. Defaults to ghcr.io/portakiapp for official modules; required
    /// for any other author.
    #[arg(long)]
    pub registry: Option<String>,
    /// Validate packaging without pushing.
    #[arg(long)]
    pub dry_run: bool,
    /// Artifact directory (defaults to `target/portaki`).
    #[arg(long)]
    pub artifact_dir: Option<PathBuf>,
    /// Skip the implicit `portaki build --release` (not recommended).
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

    let module_root = std::env::current_dir().context("current_dir")?;
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
        return announce(&args, &coords, &pushed).await;
    }

    if args.skip_build {
        ui::skipped("build skipped (--skip-build)");
    } else {
        build::run(BuildArgs {
            release: true,
            manifest_only: false,
            nested: true,
        })
        .await
        .context("portaki build --release before publish")?;
        ui::blank();
    }

    let packing = ui::step("packing the OCI artifact");
    oci::package_artifact_with_root(&module_root, &artifact_dir).context("package OCI artifact")?;
    assert_publish_version_matches_env(&module_root, &artifact_dir)?;
    packing.done("packed the OCI artifact");

    if args.dry_run {
        ui::success("dry run — nothing was pushed, nothing was announced");
        ui::field("artifact", artifact_dir.display());
        ui::field("registry", &registry);
        ui::advice("drop --dry-run to push these layers and announce the version");
        ui::blank();
        return Ok(());
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
        return Ok(());
    }

    announce(&args, &coords, &pushed).await
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
    anyhow::bail!(
        "{} {} is already in the registry ({published}) — publications are immutable, so \
         pushing again would leave the OCI tag pointing at something the catalogue does not \
         reference. Bump the version, or replay the announcement with \
         portaki publish --announce-only",
        coords.id,
        coords.version
    )
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
    let response = reqwest::Client::new()
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
                post_publication(&base, &body, &auth::refresh().await?).await?
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
    let response = reqwest::Client::new()
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
    let expected = match std::env::var("PORTAKI_PUBLISH_VERSION") {
        Ok(value) => value,
        Err(_) => return Ok(()),
    };
    let expected = expected.trim();
    if expected.is_empty() {
        return Ok(());
    }
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
        unsafe {
            std::env::set_var("PORTAKI_PUBLISH_VERSION", "0.2.1");
        }
        assert_publish_version_matches_env(root.path(), &artifact).unwrap();
        unsafe {
            std::env::remove_var("PORTAKI_PUBLISH_VERSION");
        }
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
        unsafe {
            std::env::set_var("PORTAKI_PUBLISH_VERSION", "0.2.1");
        }
        let err = assert_publish_version_matches_env(root.path(), &artifact).unwrap_err();
        assert!(err.to_string().contains("0.1.0"));
        assert!(err.to_string().contains("0.2.1"));
        unsafe {
            std::env::remove_var("PORTAKI_PUBLISH_VERSION");
        }
    }
}
