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
const SCOPES: [&str; 2] = ["modules:read", "modules:write"];

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
    ui::header("portaki login");

    let base = base_url(args.url.as_deref());
    let client = reqwest::Client::new();

    let asking = ui::step("asking the platform for a code");
    let response = client
        .post(format!("{base}/api/v1/auth/device/code"))
        .json(&serde_json::json!({ "clientId": CLIENT_ID, "scopes": SCOPES }))
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
            ui::next(&["portaki dev --watch"]);
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
pub fn logout() -> Result<()> {
    ui::header("portaki logout");
    auth::forget()?;
    ui::success("signed out — credentials cleared from the system keychain");
    ui::blank();
    Ok(())
}

fn base_url(explicit: Option<&str>) -> String {
    auth::api_base_url(explicit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cli_never_asks_for_the_host_scope() {
        // Un jeton de CLI ne fait pas d'opérations hôte ; le serveur le raboterait de toute
        // façon, mais le demander serait déjà une intention de trop.
        assert!(!SCOPES.contains(&"host"));
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
