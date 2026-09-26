//! `portaki publish --sign` — la signature d'auteur, hors CI.
//!
//! En CI, c'est l'action de release qui signe, avec provenance et audit. Depuis un poste, il ne
//! reste que l'identité de l'auteur : un certificat Fulcio éphémère pour son compte GitHub, la
//! signature au journal Rekor, déposée sur le registre OCI au format de `cosign sign` v3 — celui
//! que le registre Portaki vérifie déjà. C'est donc `cosign` lui-même qui signe.
//!
//! Le jeton OIDC, en revanche, c'est la CLI qui le demande. Le flux navigateur de cosign laisse
//! choisir le fournisseur (GitHub, Google, Microsoft) et n'offre aucun drapeau pour l'imposer ;
//! il ne dit pas non plus au nom de qui il a signé. La CLI fait donc le même flux que cosign
//! (PKCE auprès de `oauth2.sigstore.dev`) avec `connector_id` fixé à GitHub, lit l'adresse dans
//! le jeton, et le remet à cosign par `SIGSTORE_ID_TOKEN` — jamais sur la ligne de commande,
//! jamais affiché. La clé est éphémère, fabriquée et jetée par cosign.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use base64::Engine;
use oci_distribution::secrets::RegistryAuth;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::{oci, ui};

/// L'émetteur public de Sigstore (Dex), et le connecteur GitHub qu'on lui impose.
const ISSUER: &str = "https://oauth2.sigstore.dev/auth";
const GITHUB: &str = "https://github.com/login/oauth";

/// La version de l'action de release, celle dont le registre vérifie le format.
const MINIMUM: (u64, u64, u64) = (3, 1, 3);
const INSTALL: &str =
    "brew install cosign, or go install github.com/sigstore/cosign/v3/cmd/cosign@v3.1.3";

/// Le temps laissé pour se connecter dans le navigateur.
const SIGN_IN_TIMEOUT: Duration = Duration::from_secs(300);

/// `CI` ou `GITHUB_ACTIONS` posés : on est dans une CI.
pub fn in_ci() -> bool {
    ["CI", "GITHUB_ACTIONS"]
        .iter()
        .any(|name| std::env::var_os(name).is_some())
}

/// En CI, la signature revient à l'action de release : elle seule y ajoute la provenance.
pub fn refuse_in_ci(ci: bool) -> Result<()> {
    if ci {
        bail!(
            "--sign signs from a workstation, with your GitHub identity — in CI the release \
             action (PortakiApp/portaki-release-action) signs, with provenance: drop --sign"
        );
    }
    Ok(())
}

/// `PORTAKI_COSIGN`, sinon `cosign` dans le `PATH`.
pub fn cosign_binary() -> PathBuf {
    std::env::var_os("PORTAKI_COSIGN")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("cosign"))
}

/// Vérifié avant tout build : découvrir l'absence de cosign après la poussée laisserait un
/// artefact sur GHCR sans signature ni annonce.
pub fn check_cosign(bin: &Path) -> Result<()> {
    let output = match Command::new(bin).args(["version", "--json"]).output() {
        Err(failure) if failure.kind() == std::io::ErrorKind::NotFound => {
            bail!("--sign needs cosign v3.1.3 or a later 3.x, and none is installed — {INSTALL}")
        }
        other => other.context("run cosign version")?,
    };
    let found = serde_json::from_slice::<serde_json::Value>(&output.stdout)
        .ok()
        .and_then(|version| Some(version.get("gitVersion")?.as_str()?.to_string()));
    match found.as_deref().and_then(parse_version) {
        Some(version) if version.0 == MINIMUM.0 && version >= MINIMUM => Ok(()),
        _ => bail!(
            "--sign needs cosign v3.1.3 or a later 3.x — the format the registry verifies — and \
             found {} — {INSTALL}",
            found.as_deref().unwrap_or("a version it cannot read")
        ),
    }
}

fn parse_version(raw: &str) -> Option<(u64, u64, u64)> {
    let mut parts = raw.trim().trim_start_matches('v').split(['.', '-', '+']);
    Some((
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
    ))
}

/// Signe le digest poussé et rend l'adresse de l'auteur.
pub async fn sign(bin: &Path, pushed: &oci::PushedArtifact, registry: &str) -> Result<String> {
    let identity = github_identity().await?;
    let config = RegistryConfig::write(registry)?;
    let subject = subject(&pushed.image_ref, &pushed.digest);
    ui::command(
        "cosign sign",
        &mut cosign_sign(bin, &subject, &config.0, &identity.token),
    )?;
    Ok(identity.email)
}

/// `dépôt@digest` : c'est le digest poussé qu'on signe, jamais le tag.
fn subject(image_ref: &str, digest: &str) -> String {
    let repository = match image_ref.rsplit_once(':') {
        Some((repository, tag)) if !tag.contains('/') => repository,
        _ => image_ref,
    };
    format!("{repository}@{digest}")
}

/// Le jeton passe par l'environnement : sur la ligne de commande, `ps` le montrerait.
fn cosign_sign(bin: &Path, subject: &str, docker_config: &Path, token: &str) -> Command {
    let mut command = Command::new(bin);
    command
        .args(["sign", "--yes", "--oidc-provider", "envvar", subject])
        .env("SIGSTORE_ID_TOKEN", token)
        .env("DOCKER_CONFIG", docker_config);
    command
}

/// Les identifiants de la poussée, remis à cosign dans un `DOCKER_CONFIG` temporaire (0600) —
/// cosign ne lit ni `GITHUB_TOKEN` ni `GHCR_TOKEN`, et un mot de passe sur la ligne de commande
/// se lirait dans `ps`. Effacé à la sortie, quoi qu'il arrive.
struct RegistryConfig(PathBuf);

impl RegistryConfig {
    fn write(registry: &str) -> Result<Self> {
        let RegistryAuth::Basic(user, password) = oci::auth::resolve_registry_auth(registry)?
        else {
            bail!("no registry credentials for cosign — set GITHUB_TOKEN or docker login");
        };
        let dir = std::env::temp_dir().join(format!("portaki-sign-{}", uuid::Uuid::new_v4()));
        let mut builder = std::fs::DirBuilder::new();
        #[cfg(unix)]
        std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
        builder
            .create(&dir)
            .context("create a registry config for cosign")?;
        let config = Self(dir);
        let auth = base64::engine::general_purpose::STANDARD.encode(format!("{user}:{password}"));
        let body = serde_json::json!({
            "auths": { oci::auth::registry_host(registry): { "auth": auth } }
        });
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
        std::io::Write::write_all(
            &mut options.open(config.0.join("config.json"))?,
            body.to_string().as_bytes(),
        )
        .context("write the registry config for cosign")?;
        Ok(config)
    }
}

impl Drop for RegistryConfig {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct Identity {
    token: String,
    email: String,
}

/// Le flux navigateur de Sigstore, GitHub imposé : PKCE, retour sur un port local.
async fn github_identity() -> Result<Identity> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .context("open a local port for the sign-in callback")?;
    let redirect = format!(
        "http://localhost:{}/auth/callback",
        listener.local_addr()?.port()
    );
    let (verifier, state, nonce) = (random(), random(), random());
    let url = authorize_url(&redirect, &verifier, &state, &nonce);

    ui::blank();
    if ui::open_browser(&url) {
        ui::success("opened your browser — sign in with GitHub to sign this version");
    } else {
        ui::field("open", &url);
    }
    ui::blank();

    let code = tokio::time::timeout(SIGN_IN_TIMEOUT, callback(&listener, &state))
        .await
        .context("no GitHub sign-in within 5 minutes")??;
    let token = exchange(&code, &redirect, &verifier).await?;
    let email = github_email(&token, &nonce)?;
    Ok(Identity { token, email })
}

fn random() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

fn authorize_url(redirect: &str, verifier: &str, state: &str, nonce: &str) -> String {
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(Sha256::digest(verifier.as_bytes()));
    let mut url = reqwest::Url::parse(&format!("{ISSUER}/auth")).expect("static URL");
    url.query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", "sigstore")
        .append_pair("redirect_uri", redirect)
        .append_pair("scope", "openid email")
        .append_pair("state", state)
        .append_pair("nonce", nonce)
        .append_pair("code_challenge", &challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("connector_id", GITHUB);
    url.into()
}

/// Attend le retour du navigateur ; tout autre appel (favicon…) reçoit un 404.
async fn callback(listener: &tokio::net::TcpListener, state: &str) -> Result<String> {
    loop {
        let (mut stream, _) = listener.accept().await?;
        let mut buffer = vec![0; 8192];
        let read = stream.read(&mut buffer).await.unwrap_or(0);
        let request = String::from_utf8_lossy(&buffer[..read]);
        let Some(query) = callback_query(&request) else {
            let _ = stream
                .write_all(b"HTTP/1.1 404 Not Found\r\nConnection: close\r\n\r\n")
                .await;
            continue;
        };
        let _ = stream
            .write_all(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\n\r\n\
                 <p>Portaki: back to the terminal.</p>"
                    .as_bytes(),
            )
            .await;
        return read_callback(&query, state);
    }
}

fn callback_query(request: &str) -> Option<Vec<(String, String)>> {
    let target = request
        .lines()
        .next()?
        .strip_prefix("GET ")?
        .split(' ')
        .next()?;
    let url = reqwest::Url::parse(&format!("http://localhost{target}")).ok()?;
    (url.path() == "/auth/callback").then(|| url.query_pairs().into_owned().collect())
}

fn read_callback(query: &[(String, String)], state: &str) -> Result<String> {
    let get = |key: &str| {
        query
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.as_str())
    };
    if let Some(error) = get("error") {
        bail!(
            "GitHub sign-in refused: {error} {}",
            get("error_description").unwrap_or_default()
        );
    }
    if get("state") != Some(state) {
        bail!("GitHub sign-in answered for another request — start again");
    }
    get("code")
        .map(str::to_string)
        .context("GitHub sign-in returned no code")
}

async fn exchange(code: &str, redirect: &str, verifier: &str) -> Result<String> {
    #[derive(serde::Deserialize)]
    struct Tokens {
        id_token: String,
    }
    let response = crate::http::client()
        .post(format!("{ISSUER}/token"))
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect),
            ("client_id", "sigstore"),
            ("code_verifier", verifier),
        ])
        .send()
        .await
        .context("exchange the GitHub sign-in at Sigstore")?;
    if !response.status().is_success() {
        bail!(
            "Sigstore refused the GitHub sign-in ({})",
            response.status()
        );
    }
    Ok(response
        .json::<Tokens>()
        .await
        .context("read the Sigstore token")?
        .id_token)
}

/// L'adresse à afficher, et deux contrôles : le nonce, et GitHub comme fournisseur.
///
/// La signature du jeton n'est pas vérifiée ici : c'est Fulcio qui le fait avant d'émettre le
/// certificat. Ces claims ne servent qu'à dire au nom de qui on signe, et à refuser tôt.
fn github_email(token: &str, nonce: &str) -> Result<String> {
    #[derive(serde::Deserialize)]
    struct Federated {
        connector_id: String,
    }
    #[derive(serde::Deserialize)]
    struct Claims {
        email: Option<String>,
        email_verified: Option<bool>,
        nonce: Option<String>,
        federated_claims: Option<Federated>,
    }
    let payload = token
        .split('.')
        .nth(1)
        .and_then(|part| {
            base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(part)
                .ok()
        })
        .context("the Sigstore token is not a JWT")?;
    let claims: Claims =
        serde_json::from_slice(&payload).context("read the Sigstore token claims")?;
    if claims.nonce.as_deref() != Some(nonce) {
        bail!("the Sigstore token answers another request — start again");
    }
    if let Some(federated) = &claims.federated_claims {
        if federated.connector_id != GITHUB {
            bail!(
                "signed in with {} — only a GitHub identity signs a Portaki module",
                federated.connector_id
            );
        }
    }
    match (claims.email, claims.email_verified) {
        (Some(email), Some(true)) => Ok(email),
        _ => bail!("your GitHub account has no verified email — Fulcio needs one to sign"),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Un faux cosign : un script qui écrit `stdout` et sort avec `code`.
    #[cfg(unix)]
    pub(crate) fn fake_cosign(dir: &Path, stdout: &str, code: i32) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join("cosign");
        std::fs::write(
            &path,
            format!("#!/bin/sh\nprintf '%s' '{stdout}'\nexit {code}\n"),
        )
        .unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    #[test]
    fn ci_refuses_and_points_at_the_release_action() {
        let refusal = refuse_in_ci(true).unwrap_err().to_string();
        assert!(refusal.contains("portaki-release-action"));
        assert!(refuse_in_ci(false).is_ok());
    }

    #[test]
    fn the_pushed_digest_is_signed_never_the_tag() {
        assert_eq!(
            subject("ghcr.io/acme/portaki-modules-nuki:1.4.0", "sha256:9f2c"),
            "ghcr.io/acme/portaki-modules-nuki@sha256:9f2c"
        );
        assert_eq!(
            subject("localhost:5000/portaki-modules-nuki:1.4.0", "sha256:9f2c"),
            "localhost:5000/portaki-modules-nuki@sha256:9f2c"
        );
    }

    #[test]
    fn the_command_is_cosign_sign_with_the_token_kept_off_argv() {
        let command = cosign_sign(
            Path::new("/opt/cosign"),
            "ghcr.io/acme/portaki-modules-nuki@sha256:9f2c",
            Path::new("/tmp/cfg"),
            "secret.jwt.token",
        );
        assert_eq!(command.get_program(), "/opt/cosign");
        let args: Vec<_> = command.get_args().collect();
        assert_eq!(
            args,
            [
                "sign",
                "--yes",
                "--oidc-provider",
                "envvar",
                "ghcr.io/acme/portaki-modules-nuki@sha256:9f2c"
            ]
        );
        let envs: Vec<_> = command.get_envs().collect();
        assert!(envs.contains(&(
            std::ffi::OsStr::new("SIGSTORE_ID_TOKEN"),
            Some(std::ffi::OsStr::new("secret.jwt.token"))
        )));
        assert!(envs.contains(&(
            std::ffi::OsStr::new("DOCKER_CONFIG"),
            Some(std::ffi::OsStr::new("/tmp/cfg"))
        )));
    }

    #[cfg(unix)]
    #[test]
    fn cosign_3_1_3_or_a_later_3_x_is_accepted_and_nothing_else() {
        let dir = tempfile::tempdir().unwrap();
        for (version, ok) in [
            ("v3.1.3", true),
            ("v3.2.0", true),
            ("v3.1.2", false),
            ("v2.4.1", false),
            ("v4.0.0", false),
        ] {
            let bin = fake_cosign(dir.path(), &format!(r#"{{"gitVersion":"{version}"}}"#), 0);
            assert_eq!(check_cosign(&bin).is_ok(), ok, "{version}");
        }
    }

    #[test]
    fn a_missing_cosign_says_how_to_install_it() {
        let refusal = check_cosign(Path::new("/nonexistent/cosign"))
            .unwrap_err()
            .to_string();
        assert!(refusal.contains("brew install cosign"), "{refusal}");
    }

    #[cfg(unix)]
    #[test]
    fn a_failing_cosign_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let bin = fake_cosign(dir.path(), "", 1);
        let mut command = cosign_sign(&bin, "r@sha256:1", dir.path(), "t");
        assert!(ui::command("cosign sign", &mut command).is_err());
    }

    #[test]
    fn the_sign_in_asks_sigstore_for_github_with_pkce() {
        let url = authorize_url("http://localhost:1/auth/callback", "v", "s", "n");
        assert!(url.starts_with("https://oauth2.sigstore.dev/auth/auth?"));
        assert!(url.contains("connector_id=https%3A%2F%2Fgithub.com%2Flogin%2Foauth"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(!url.contains("code_challenge=v&"));
    }

    #[test]
    fn the_callback_checks_the_state() {
        let query = callback_query("GET /auth/callback?code=c&state=s HTTP/1.1\r\n").unwrap();
        assert_eq!(read_callback(&query, "s").unwrap(), "c");
        assert!(read_callback(&query, "other").is_err());
        assert!(callback_query("GET /favicon.ico HTTP/1.1\r\n").is_none());
    }

    fn jwt(claims: serde_json::Value) -> String {
        let engine = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        format!("h.{}.s", engine.encode(claims.to_string()))
    }

    #[test]
    fn only_a_verified_github_identity_signs() {
        let github = jwt(serde_json::json!({
            "email": "dev@example.com", "email_verified": true, "nonce": "n",
            "federated_claims": { "connector_id": GITHUB }
        }));
        assert_eq!(github_email(&github, "n").unwrap(), "dev@example.com");
        assert!(github_email(&github, "other").is_err());

        let google = jwt(serde_json::json!({
            "email": "dev@example.com", "email_verified": true, "nonce": "n",
            "federated_claims": { "connector_id": "https://accounts.google.com" }
        }));
        assert!(github_email(&google, "n").is_err());

        let unverified = jwt(serde_json::json!({ "email": "dev@example.com", "nonce": "n" }));
        assert!(github_email(&unverified, "n").is_err());
    }
}
