//! How `portaki release` signs by default: keyless (Sigstore), over the digest pushed into
//! Portaki's OCI repository, with the ephemeral credentials of the push grant.
//!
//! **In CI** (`portaki ci release`), the identity is the job's OIDC token: two `cosign attest`
//! attestations — the SLSA v1 provenance (repository, commit, workflow) and the `cargo audit`
//! report ([`AUDIT_TYPE`]). The Fulcio certificate itself carries the workflow, the repository
//! and the commit: that is what the registry checks against the module's binding.
//!
//! **From a workstation**, all that is left is the author's identity: an ephemeral Fulcio
//! certificate for their GitHub account, and a Portaki attestation ([`PREDICATE_TYPE`]) in the
//! Rekor log and then on the OCI repository. This is not a provenance: it says who published
//! which module, in which version, nothing about the way it was built.
//!
//! Not `cosign sign`: its in-toto subject has no `name`, and sigstore-java, on the registry side,
//! does not read it. `cosign attest` names the subject after the repository, next to the digest.
//!
//! A workstation's OIDC token is asked for by the CLI itself. Cosign's browser flow lets the
//! provider be chosen (GitHub, Google, Microsoft) and offers no flag to force one; nor does it
//! say on whose behalf it signed. The CLI therefore runs the same flow as cosign (PKCE against
//! `oauth2.sigstore.dev`) with `connector_id` pinned to GitHub, reads the address from the token,
//! and hands it to cosign through `SIGSTORE_ID_TOKEN` — never on the command line, never
//! displayed. The key is ephemeral, made and thrown away by cosign.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use base64::Engine;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::{oci, ui};

/// Sigstore's public issuer (Dex), and the GitHub connector we force on it.
const ISSUER: &str = "https://oauth2.sigstore.dev/auth";
const GITHUB: &str = "https://github.com/login/oauth";

/// The predicate type of the author attestation, the one the registry expects.
const PREDICATE_TYPE: &str = "https://portaki.app/attestations/author/v1";

/// The SLSA v1 provenance (`https://slsa.dev/provenance/v1`), under cosign's short name.
const PROVENANCE_TYPE: &str = "slsaprovenance1";

/// The `cargo audit` report, which the registry reads alongside the provenance.
pub const AUDIT_TYPE: &str = "https://portaki.app/attestations/cargo-audit/v1";

/// The release action's version, the one whose format the registry verifies.
const MINIMUM: (u64, u64, u64) = (3, 1, 3);
const INSTALL: &str =
    "brew install cosign, or go install github.com/sigstore/cosign/v3/cmd/cosign@v3.1.3";

/// The time allowed to sign in in the browser.
const SIGN_IN_TIMEOUT: Duration = Duration::from_secs(300);

/// `CI` or `GITHUB_ACTIONS` set: we are in a CI.
pub fn in_ci() -> bool {
    ["CI", "GITHUB_ACTIONS"]
        .iter()
        .any(|name| std::env::var_os(name).is_some())
}

/// The author signature needs a browser: in CI, it is `portaki ci release` that signs, with the
/// workflow's provenance.
pub fn refuse_in_ci(ci: bool) -> Result<()> {
    if ci {
        bail!(
            "portaki release signs with your GitHub identity, from a workstation — in CI, build \
             with portaki ci build and publish with portaki ci release (or \
             PortakiApp/portaki-release-action@v2), which signs with the workflow's provenance; \
             --no-sign publishes unsigned, for the sandbox only"
        );
    }
    Ok(())
}

/// `PORTAKI_COSIGN`, otherwise `cosign` from the `PATH`.
pub fn cosign_binary() -> PathBuf {
    std::env::var_os("PORTAKI_COSIGN")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("cosign"))
}

/// Checked before any build: discovering that cosign is missing after the push would leave an
/// artifact pushed with neither a signature nor an announcement.
pub fn check_cosign(bin: &Path) -> Result<()> {
    let output = match Command::new(bin).args(["version", "--json"]).output() {
        Err(failure) if failure.kind() == std::io::ErrorKind::NotFound => {
            bail!("signing needs cosign v3.1.3 or a later 3.x, and none is installed — {INSTALL} (or --no-sign: sandbox only)")
        }
        other => other.context("run cosign version")?,
    };
    let found = serde_json::from_slice::<serde_json::Value>(&output.stdout)
        .ok()
        .and_then(|version| Some(version.get("gitVersion")?.as_str()?.to_string()));
    match found.as_deref().and_then(parse_version) {
        Some(version) if version.0 == MINIMUM.0 && version >= MINIMUM => Ok(()),
        _ => bail!(
            "signing needs cosign v3.1.3 or a later 3.x — the format the registry verifies — and \
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

/// Attests the pushed digest in the author's name and returns their address.
pub async fn sign(
    bin: &Path,
    pushed: &oci::PushedArtifact,
    grant: &oci::PushGrant,
    coords: &oci::pack::ModuleCoordinates,
) -> Result<String> {
    let identity = github_identity().await?;
    let scratch = Scratch::new()?;
    scratch.write_registry_config(grant)?;
    let predicate = scratch.write(
        "predicate.json",
        serde_json::json!({ "moduleId": coords.id, "version": coords.version })
            .to_string()
            .as_bytes(),
    )?;
    let mut command = cosign_attest(
        bin,
        PREDICATE_TYPE,
        &predicate,
        pushed,
        &scratch.0,
        "envvar",
    );
    command.env("SIGSTORE_ID_TOKEN", &identity.token);
    ui::command("cosign attest", &mut command)?;
    Ok(identity.email)
}

/// Attests the pushed digest from CI: the workflow's provenance, then the audit report if there
/// is one. Returns true when the audit is attested.
///
/// The identity is this job's OIDC token (`id-token: write`), which cosign asks GitHub for
/// itself. No module code runs here.
pub fn attest_ci(
    bin: &Path,
    pushed: &oci::PushedArtifact,
    grant: &oci::PushGrant,
    audit: Option<&Path>,
) -> Result<bool> {
    let scratch = Scratch::new()?;
    scratch.write_registry_config(grant)?;
    let provenance = scratch.write(
        "provenance.json",
        provenance(&|name| std::env::var(name).ok())
            .to_string()
            .as_bytes(),
    )?;
    let mut command = cosign_attest(bin, PROVENANCE_TYPE, &provenance, pushed, &scratch.0, CI);
    ui::command("cosign attest (provenance)", &mut command)?;

    let Some(audit) = audit else {
        return Ok(false);
    };
    let mut command = cosign_attest(bin, AUDIT_TYPE, audit, pushed, &scratch.0, CI);
    ui::command("cosign attest (audit)", &mut command)?;
    Ok(true)
}

/// The SLSA v1 provenance of this run, from what GitHub Actions puts in the environment.
///
/// The registry does not take this content on trust: it verifies the certificate, which carries
/// the same facts signed by GitHub. The predicate makes them readable to whoever re-reads the
/// attestation.
fn provenance(env: &dyn Fn(&str) -> Option<String>) -> serde_json::Value {
    let get = |name: &str| env(name).unwrap_or_default();
    let server = env("GITHUB_SERVER_URL").unwrap_or_else(|| "https://github.com".to_string());
    let repository = get("GITHUB_REPOSITORY");
    let workflow_ref = get("GITHUB_WORKFLOW_REF");
    let path = workflow_ref
        .strip_prefix(&format!("{repository}/"))
        .unwrap_or(&workflow_ref);
    let path = path.split_once('@').map_or(path, |(path, _)| path);
    let reference = get("GITHUB_REF");
    serde_json::json!({
        "buildDefinition": {
            "buildType": "https://portaki.app/buildtypes/release-action/v2",
            "externalParameters": {
                "workflow": { "repository": format!("{server}/{repository}"), "path": path, "ref": reference }
            },
            "internalParameters": {
                "github": {
                    "event_name": get("GITHUB_EVENT_NAME"),
                    "repository_id": get("GITHUB_REPOSITORY_ID"),
                    "repository_owner_id": get("GITHUB_REPOSITORY_OWNER_ID"),
                }
            },
            "resolvedDependencies": [{
                "uri": format!("git+{server}/{repository}@{reference}"),
                "digest": { "gitCommit": get("GITHUB_SHA") }
            }]
        },
        "runDetails": {
            "builder": { "id": format!("{server}/{workflow_ref}") },
            "metadata": {
                "invocationId": format!(
                    "{server}/{repository}/actions/runs/{}/attempts/{}",
                    get("GITHUB_RUN_ID"),
                    get("GITHUB_RUN_ATTEMPT")
                )
            }
        }
    })
}

/// Cosign's identity provider inside a GitHub Actions job: the job's OIDC token.
const CI: &str = "github-actions";

/// `cosign attest` on the pushed digest, with the push grant's credentials in an ephemeral
/// `DOCKER_CONFIG`: neither token nor password on the command line, where `ps` would show them.
fn cosign_attest(
    bin: &Path,
    predicate_type: &str,
    predicate: &Path,
    pushed: &oci::PushedArtifact,
    docker_config: &Path,
    oidc_provider: &str,
) -> Command {
    let mut command = Command::new(bin);
    command
        .args(["attest", "--yes", "--oidc-provider", oidc_provider])
        .args(["--type", predicate_type])
        .arg("--predicate")
        .arg(predicate)
        .env("DOCKER_CONFIG", docker_config);
    if oci::is_local(&pushed.registry) {
        command.arg("--allow-http-registry");
    }
    command.arg(pushed.subject());
    command
}

/// A temporary directory (0700) for cosign: the `DOCKER_CONFIG` and the predicate, each in 0600.
/// The push grant goes through it because cosign pushes its attestations into the same
/// repository, and because a password on the command line would be readable in `ps`. Deleted on
/// the way out, whatever happens.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Result<Self> {
        let dir = std::env::temp_dir().join(format!("portaki-sign-{}", uuid::Uuid::new_v4()));
        let mut builder = std::fs::DirBuilder::new();
        #[cfg(unix)]
        std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
        builder
            .create(&dir)
            .context("create a temporary directory for cosign")?;
        Ok(Self(dir))
    }

    fn write(&self, name: &str, body: &[u8]) -> Result<PathBuf> {
        let path = self.0.join(name);
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
        std::io::Write::write_all(&mut options.open(&path)?, body)
            .with_context(|| format!("write {name} for cosign"))?;
        Ok(path)
    }

    fn write_registry_config(&self, grant: &oci::PushGrant) -> Result<()> {
        let auth = base64::engine::general_purpose::STANDARD
            .encode(format!("{}:{}", grant.username, grant.password));
        let body = serde_json::json!({ "auths": { &grant.registry: { "auth": auth } } });
        self.write("config.json", body.to_string().as_bytes())?;
        Ok(())
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct Identity {
    token: String,
    email: String,
}

/// Sigstore's browser flow, with GitHub forced: PKCE, callback on a local port.
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

/// Waits for the browser to come back; any other call (favicon…) gets a 404.
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

/// The address to display, and two checks: the nonce, and GitHub as the provider.
///
/// The token's signature is not verified here: Fulcio is the one that does it, before issuing the
/// certificate. These claims only serve to say on whose behalf we are signing, and to refuse
/// early.
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

    /// A fake cosign: a script that writes `stdout` and exits with `code`.
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
    fn ci_refuses_and_points_at_ci_release() {
        let refusal = refuse_in_ci(true).unwrap_err().to_string();
        assert!(refusal.contains("portaki ci release"), "{refusal}");
        assert!(refusal.contains("--no-sign"), "{refusal}");
        assert!(refuse_in_ci(false).is_ok());
    }

    pub(crate) fn pushed(registry: &str) -> oci::PushedArtifact {
        oci::PushedArtifact {
            registry: registry.to_string(),
            repository: "modules/nuki".to_string(),
            digest: "sha256:9f2c".to_string(),
        }
    }

    pub(crate) fn grant(registry: &str) -> oci::PushGrant {
        serde_json::from_value(serde_json::json!({
            "registry": registry,
            "repository": "modules/nuki",
            "reference": format!("{registry}/modules/nuki:1.4.0"),
            "username": "portaki-push",
            "password": "pk_push_secret",
            "expiresAt": "2026-09-27T12:00:00Z",
        }))
        .unwrap()
    }

    #[test]
    fn the_pushed_digest_is_attested_never_the_tag() {
        let command = cosign_attest(
            Path::new("/opt/cosign"),
            PREDICATE_TYPE,
            Path::new("/tmp/cfg/predicate.json"),
            &pushed("oci.portaki.app"),
            Path::new("/tmp/cfg"),
            "envvar",
        );
        assert_eq!(command.get_program(), "/opt/cosign");
        let args: Vec<_> = command.get_args().collect();
        assert_eq!(
            args,
            [
                "attest",
                "--yes",
                "--oidc-provider",
                "envvar",
                "--type",
                "https://portaki.app/attestations/author/v1",
                "--predicate",
                "/tmp/cfg/predicate.json",
                "oci.portaki.app/modules/nuki@sha256:9f2c"
            ]
        );
        let envs: Vec<_> = command.get_envs().collect();
        assert!(envs.contains(&(
            std::ffi::OsStr::new("DOCKER_CONFIG"),
            Some(std::ffi::OsStr::new("/tmp/cfg"))
        )));
    }

    #[test]
    fn a_development_repository_is_attested_over_http() {
        let command = cosign_attest(
            Path::new("cosign"),
            PREDICATE_TYPE,
            Path::new("p.json"),
            &pushed("oci.localhost:8080"),
            Path::new("/tmp/cfg"),
            CI,
        );
        assert!(command.get_args().any(|arg| arg == "--allow-http-registry"));
    }

    /// Two attestations on the digest, the provenance first; the push grant is only in the
    /// ephemeral `DOCKER_CONFIG`, for the host the registry named.
    #[cfg(unix)]
    #[test]
    fn ci_attests_provenance_then_audit_with_the_push_grant() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join("calls");
        let bin = dir.path().join("cosign");
        std::fs::write(
            &bin,
            format!(
                "#!/bin/sh\necho \"$*\" >>{log}\ncat \"$DOCKER_CONFIG/config.json\" >>{log}\necho >>{log}\n",
                log = log.display()
            ),
        )
        .unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        let audit = dir.path().join("cargo-audit.json");
        std::fs::write(&audit, "{}").unwrap();
        let registry = "oci-staging.portaki.app";

        let audited = attest_ci(&bin, &pushed(registry), &grant(registry), Some(&audit)).unwrap();

        assert!(audited);
        let calls = std::fs::read_to_string(&log).unwrap();
        let lines: Vec<&str> = calls
            .lines()
            .filter(|line| line.starts_with("attest"))
            .collect();
        assert_eq!(lines.len(), 2, "{calls}");
        assert!(lines[0].contains("--type slsaprovenance1"), "{calls}");
        assert!(
            lines[1].contains(&format!("--type {AUDIT_TYPE}")),
            "{calls}"
        );
        for line in &lines {
            assert!(
                line.ends_with("oci-staging.portaki.app/modules/nuki@sha256:9f2c"),
                "{line}"
            );
            assert!(line.contains("--oidc-provider github-actions"), "{line}");
            assert!(!line.contains("pk_push_secret"), "{line}");
        }
        let auth = base64::engine::general_purpose::STANDARD.encode("portaki-push:pk_push_secret");
        assert!(
            calls.contains(&format!(
                r#"{{"auths":{{"{registry}":{{"auth":"{auth}"}}}}}}"#
            )),
            "{calls}"
        );
    }

    #[test]
    fn provenance_names_the_workflow_file_and_the_commit() {
        let env = |name: &str| {
            Some(
                match name {
                    "GITHUB_REPOSITORY" => "PortakiApp/portaki-modules",
                    "GITHUB_WORKFLOW_REF" => {
                        "PortakiApp/portaki-modules/.github/workflows/ci.yml@refs/heads/main"
                    }
                    "GITHUB_REF" => "refs/heads/main",
                    "GITHUB_SHA" => "0123",
                    "GITHUB_RUN_ID" => "7",
                    "GITHUB_RUN_ATTEMPT" => "1",
                    _ => return None,
                }
                .to_string(),
            )
        };

        let predicate = provenance(&env);

        let workflow = &predicate["buildDefinition"]["externalParameters"]["workflow"];
        assert_eq!(workflow["path"], ".github/workflows/ci.yml");
        assert_eq!(
            workflow["repository"],
            "https://github.com/PortakiApp/portaki-modules"
        );
        assert_eq!(
            predicate["buildDefinition"]["resolvedDependencies"][0]["digest"]["gitCommit"],
            "0123"
        );
        assert_eq!(
            predicate["runDetails"]["builder"]["id"],
            "https://github.com/PortakiApp/portaki-modules/.github/workflows/ci.yml@refs/heads/main"
        );
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
        let registry = "oci.portaki.app";
        assert!(attest_ci(&bin, &pushed(registry), &grant(registry), None).is_err());
    }

    #[test]
    fn the_predicate_is_private_and_gone_afterwards() {
        let scratch = Scratch::new().unwrap();
        let dir = scratch.0.clone();
        let predicate = scratch
            .write(
                "predicate.json",
                br#"{"moduleId":"nuki","version":"1.4.0"}"#,
            )
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(&predicate).unwrap(),
            r#"{"moduleId":"nuki","version":"1.4.0"}"#
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&predicate).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        drop(scratch);
        assert!(!dir.exists());
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
