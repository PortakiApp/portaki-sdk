//! `portaki login` — device grant, RFC 8628.
//!
//! The CLI has no browser, so it asks for a code, the developer approves it from a session that
//! is already signed in, and the CLI polls until the answer comes. The model is `gh auth login`;
//! improvised by hand this flow is shaky, so it follows the spec.

use std::time::Duration;

use anyhow::{bail, Context, Result};
use clap::Parser;

use crate::{auth, ui};

const CLIENT_ID: &str = "portaki-cli";

/// What the CLI may ask for. Narrowed server-side to what this client is allowed.
///
/// One entry per thing the CLI actually does, so the approval screen can show them one by one:
/// asking for a single coarse scope would put "grant developer access" in front of the person
/// deciding, which is not something anyone can weigh. Nothing here touches host data — no
/// `host:` scope is grantable to this client, and the server would strip one anyway.
const SCOPES: [&str; 5] = [
    "dev:read",
    "dev:deploy",
    "dev:dispatch",
    "dev:stay:read",
    "registry:publish",
];

/// This binary's version, and the SDK it was built against.
///
/// Sent with the device code request so the approval screen can show which build is asking.
/// The two travel separately because they can differ: a machine may run an old `portaki`
/// against a freshly published SDK, and the person approving should see both numbers.
const CLIENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const SDK_VERSION: &str = portaki_sdk::VERSION;

#[derive(Debug, Parser)]
/// Arguments for `portaki login`.
pub struct LoginArgs {
    /// Base URL of the platform. Defaults to PORTAKI_API_URL, then production.
    #[arg(long)]
    pub url: Option<String>,

    /// Print the URL instead of opening it — for a remote shell or a headless box.
    #[arg(long)]
    pub no_browser: bool,
}

#[derive(Debug, Parser)]
/// Arguments for `portaki logout`.
pub struct LogoutArgs {
    /// Base URL of the platform. Defaults to PORTAKI_API_URL, then production.
    #[arg(long)]
    pub url: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeviceCode {
    device_code: String,
    user_code: String,
    verification_uri: String,
    /// L'URL avec le code déjà dedans (RFC 8628 §3.3.1). Le serveur n'est pas tenu de la rendre,
    /// et la fabriquer soi-même serait deviner la forme d'un paramètre : quand elle est là, il
    /// n'y a plus rien à recopier ; quand elle manque, on ouvre l'URL nue et le code s'affiche.
    #[serde(default)]
    verification_uri_complete: Option<String>,
    expires_in: u64,
    interval: u64,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Granted {
    access_token: String,
    refresh_token: String,
    #[serde(default)]
    scopes: Vec<String>,
}

/// Runs `portaki login`.
pub async fn run(args: LoginArgs) -> Result<()> {
    ui::header(
        "portaki login",
        "Device grant — the code below ties this terminal to your Portaki account.",
    );

    let base = base_url(args.url.as_deref());
    let client = reqwest::Client::new();

    let asking = ui::step("asking the platform for a code");
    // What this machine says about itself. None of it proves anything — a hostile client would
    // lie — but the approval screen has nothing else to show, and a developer recognises their
    // own machine name at a glance. Absent fields simply render as unknown.
    let response = client
        .post(format!("{base}/api/v1/auth/device/code"))
        .json(&serde_json::json!({
            "clientId": CLIENT_ID,
            "scopes": SCOPES,
            "deviceLabel": device_label(),
            "clientVersion": CLIENT_VERSION,
            "sdkVersion": SDK_VERSION,
        }))
        .send()
        .await
        .map_err(|failure| {
            asking.abandon();
            failure
        })
        .context("ask the platform for a device code")?;
    let started: DeviceCode = crate::api::unwrap(&response.text().await.unwrap_or_default())?;
    asking.done("got a code");

    present(&started, args.no_browser);

    // Le serveur dicte l'intervalle : la spec veut qu'il puisse ralentir un client trop pressé.
    let mut interval = Duration::from_secs(started.interval.max(1));
    let deadline = std::time::Instant::now() + Duration::from_secs(started.expires_in);
    let waiting = ui::step("waiting for approval");

    loop {
        let now = std::time::Instant::now();
        if now >= deadline {
            waiting.abandon();
            bail!("the code expired before it was approved — run `portaki login` again");
        }
        waiting.say(format!(
            "waiting for approval — {} left",
            ui::countdown(deadline - now)
        ));
        tokio::time::sleep(interval).await;

        let response = client
            .post(format!("{base}/api/v1/auth/device/token"))
            .json(&serde_json::json!({ "deviceCode": started.device_code }))
            .send()
            .await
            .context("poll the platform")?;

        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        if status.is_success() {
            let granted: Granted = crate::api::unwrap(&body)?;
            auth::store(&granted.access_token, &granted.refresh_token)?;
            waiting.done("approved");
            ui::success("signed in — token stored in the system keychain");
            if !granted.scopes.is_empty() {
                ui::field("scopes", granted.scopes.join(" "));
            }
            ui::advice(
                "the access token lasts minutes and renews itself — the session lives in the \
                 keychain until portaki logout",
            );
            ui::next(&[
                (
                    "portaki dev --watch",
                    "build, deploy to the sandbox, redeploy on every save",
                ),
                (
                    "portaki publish",
                    "push a release and announce it to the registry",
                ),
            ]);
            ui::blank();
            return Ok(());
        }

        // Les codes de la spec arrivent dans le `error_code` de l'enveloppe maison. Lus à plat,
        // ils ressemblaient à une réponse inconnue et la CLI abandonnait dès le premier sondage.
        let error = crate::api::error_code(&body).unwrap_or_else(|| body.clone());

        match error.as_str() {
            // Ni l'un ni l'autre n'est un échec : « pas encore » et « moins vite ».
            "authorization_pending" => {}
            "slow_down" => interval += Duration::from_secs(5),
            "access_denied" => {
                waiting.abandon();
                bail!("the request was denied");
            }
            "expired_token" => {
                waiting.abandon();
                bail!("the code expired — run `portaki login` again");
            }
            other => {
                waiting.abandon();
                bail!("the platform answered {other}");
            }
        }
    }
}

/// Montre le code, puis emmène l'utilisateur là où il l'approuve.
///
/// Le navigateur s'ouvre sur l'URL pré-remplie quand le serveur en donne une : il ne reste alors
/// qu'à confirmer. Le code reste affiché quoi qu'il arrive — c'est le seul recours si l'ouverture
/// échoue, si la CLI tourne dans un SSH, ou si le navigateur ouvert n'est pas celui où la session
/// est déjà ouverte.
fn present(started: &DeviceCode, no_browser: bool) {
    let target = started
        .verification_uri_complete
        .as_deref()
        .unwrap_or(&started.verification_uri);

    ui::blank();
    ui::code_block(&started.user_code);
    ui::blank();

    if no_browser || !ui::open_browser(target) {
        ui::field("open", target);
        ui::field("code", &started.user_code);
    } else {
        ui::success(if started.verification_uri_complete.is_some() {
            "opened your browser — check the code above, then approve"
        } else {
            "opened your browser — paste the code above to approve"
        });
        ui::detail(target);
    }
    ui::blank();
}

/// Runs `portaki logout`.
pub async fn logout(args: LogoutArgs) -> Result<()> {
    ui::header(
        "portaki logout",
        "End the session here, and on the platform.",
    );

    let stored = auth::refresh_token();

    // Effacé d'abord, quoi qu'il arrive ensuite : une déconnexion qui laisse les identifiants
    // en place parce que le réseau a hoqueté serait la pire des deux moitiés — on croit être
    // sorti, et on ne l'est nulle part.
    auth::forget()?;
    ui::success("signed out here — credentials cleared");

    let Some(refresh_token) = stored else {
        ui::detail("no session was stored");
        ui::blank();
        return Ok(());
    };

    match revoke(&base_url(args.url.as_deref()), &refresh_token).await {
        Ok(()) => ui::success("the platform revoked this session"),
        Err(failure) => {
            // Le dire, parce que c'est la moitié qui protège : un jeton non révoqué reste
            // utilisable par qui détient une copie du fichier.
            ui::warn("could not reach the platform — this session is still valid there");
            ui::detail(format!("{failure:#}"));
            ui::advice("run portaki logout again once you are online");
        }
    }
    ui::blank();
    Ok(())
}

/// Dit à la plateforme d'oublier ce jeton.
///
/// Sans quoi `portaki logout` n'efface qu'un fichier : le jeton de rafraîchissement reste
/// valide jusqu'à son expiration, et qui détient une copie du fichier reste connecté.
async fn revoke(base: &str, refresh_token: &str) -> Result<()> {
    let response = reqwest::Client::new()
        .post(format!("{base}/api/v1/auth/logout"))
        .json(&serde_json::json!({ "refreshToken": refresh_token }))
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .context("tell the platform to end this session")?;

    let status = response.status();
    if !status.is_success() {
        anyhow::bail!("the platform answered {status}");
    }
    Ok(())
}

fn base_url(explicit: Option<&str>) -> String {
    auth::api_base_url(explicit)
}

/// This machine's name, the way its owner would recognise it.
///
/// No dependency for it: `COMPUTERNAME` on Windows, the `hostname` binary everywhere else, then
/// the environment as a last resort — zsh exports `HOST`, bash exports `HOSTNAME`, and neither
/// is guaranteed. Returning `None` is a normal outcome, not a failure: the approval screen shows
/// one fewer line and login proceeds.
fn device_label() -> Option<String> {
    if let Ok(name) = std::env::var("COMPUTERNAME") {
        if let Some(name) = non_empty(name) {
            return Some(name);
        }
    }
    if let Ok(output) = std::process::Command::new("hostname").output() {
        if output.status.success() {
            if let Some(name) = non_empty(String::from_utf8_lossy(&output.stdout).into_owned()) {
                return Some(name);
            }
        }
    }
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("HOST"))
        .ok()
        .and_then(non_empty)
}

fn non_empty(value: String) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cli_never_asks_for_a_host_scope() {
        // Un jeton de CLI ne fait pas d'opérations hôte ; le serveur le raboterait de toute
        // façon, mais le demander serait déjà une intention de trop. Écrit sur le préfixe et
        // non sur un scope nommé : `host:billing` ajouté demain doit échouer ici aussi.
        assert!(!SCOPES.iter().any(|s| s.starts_with("host:")));
    }

    /// Les séjours de la sandbox et ceux d'un vrai voyageur ne portent pas le même scope.
    #[test]
    fn sandbox_stays_are_asked_for_under_the_dev_domain() {
        assert!(SCOPES.contains(&"dev:stay:read"));
        assert!(!SCOPES.contains(&"stay:read"));
    }

    /// A hostname reaches an approval screen, so it must never arrive as a raw command output —
    /// `hostname` ends its line with a newline, and a trailing one would render as a blank row.
    #[test]
    fn a_device_label_is_trimmed_or_absent() {
        assert_eq!(
            non_empty("  my-laptop.local\n".to_owned()).as_deref(),
            Some("my-laptop.local")
        );
        assert_eq!(non_empty("   ".to_owned()), None);
        assert_eq!(non_empty(String::new()), None);
    }

    /// Nothing about login depends on knowing the machine name; it only makes the screen poorer.
    #[test]
    fn a_missing_device_label_is_not_an_error() {
        let label = device_label();

        assert!(label
            .as_deref()
            .map(str::trim)
            .map(|l| !l.is_empty())
            .unwrap_or(true));
    }

    #[test]
    fn the_versions_announced_are_the_ones_compiled_in() {
        assert!(!CLIENT_VERSION.is_empty());
        assert_eq!(SDK_VERSION, portaki_sdk::VERSION);
    }

    #[test]
    fn an_explicit_url_wins_over_the_environment() {
        std::env::set_var("PORTAKI_API_URL", "https://from-env.example");

        assert_eq!(
            base_url(Some("https://explicit.example/")),
            "https://explicit.example"
        );

        std::env::remove_var("PORTAKI_API_URL");
    }

    #[test]
    fn a_trailing_slash_never_doubles_in_the_path() {
        assert_eq!(
            base_url(Some("https://api.example/")),
            "https://api.example"
        );
    }
}
