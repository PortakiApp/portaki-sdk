//! `portaki login` — device grant, RFC 8628.
//!
//! The CLI has no browser, so it asks for a code, the developer approves it from a session that
//! is already signed in, and the CLI polls until the answer comes. The model is `gh auth login`;
//! improvised by hand this flow is shaky, so it follows the spec.

use std::time::Duration;

use anyhow::{bail, Context, Result};
use clap::Parser;

use crate::auth;

const CLIENT_ID: &str = "portaki-cli";

/// What the CLI may ask for. Narrowed server-side to what this client is allowed.
const SCOPES: [&str; 2] = ["modules:read", "modules:write"];

#[derive(Debug, Parser)]
/// Arguments for `portaki login`.
pub struct LoginArgs {
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

#[derive(Debug, serde::Deserialize)]
struct OAuthError {
    error: String,
}

/// Runs `portaki login`.
pub async fn run(args: LoginArgs) -> Result<()> {
    let base = base_url(args.url.as_deref());
    let client = reqwest::Client::new();

    let started: DeviceCode = client
        .post(format!("{base}/api/v1/auth/device/code"))
        .json(&serde_json::json!({ "clientId": CLIENT_ID, "scopes": SCOPES }))
        .send()
        .await
        .context("ask the platform for a device code")?
        .json()
        .await
        .context("unexpected answer to the device code request")?;

    println!("\n  open  {}", started.verification_uri);
    println!("  code  {}\n", started.user_code);
    println!("waiting for approval…");

    // Le serveur dicte l'intervalle : la spec veut qu'il puisse ralentir un client trop pressé.
    let mut interval = Duration::from_secs(started.interval.max(1));
    let deadline = std::time::Instant::now() + Duration::from_secs(started.expires_in);

    loop {
        if std::time::Instant::now() >= deadline {
            bail!("the code expired before it was approved — run `portaki login` again");
        }
        tokio::time::sleep(interval).await;

        let response = client
            .post(format!("{base}/api/v1/auth/device/token"))
            .json(&serde_json::json!({ "deviceCode": started.device_code }))
            .send()
            .await
            .context("poll the platform")?;

        if response.status().is_success() {
            let granted: Granted = response.json().await.context("unexpected token answer")?;
            auth::store(&granted.access_token, &granted.refresh_token)?;
            println!("\nsigned in — token stored in the system keychain");
            if !granted.scopes.is_empty() {
                println!("scopes: {}", granted.scopes.join(" "));
            }
            return Ok(());
        }

        let body = response.text().await.unwrap_or_default();
        let error = serde_json::from_str::<OAuthError>(&body)
            .map(|parsed| parsed.error)
            .unwrap_or_else(|_| body.clone());

        match error.as_str() {
            // Ni l'un ni l'autre n'est un échec : « pas encore » et « moins vite ».
            "authorization_pending" => {}
            "slow_down" => interval += Duration::from_secs(5),
            "access_denied" => bail!("the request was denied"),
            "expired_token" => bail!("the code expired — run `portaki login` again"),
            other => bail!("the platform answered {other}"),
        }
    }
}

/// Runs `portaki logout`.
pub fn logout() -> Result<()> {
    auth::forget()?;
    println!("signed out — credentials cleared from the system keychain");
    Ok(())
}

fn base_url(explicit: Option<&str>) -> String {
    let raw = explicit
        .map(str::to_owned)
        .or_else(|| std::env::var("PORTAKI_API_URL").ok())
        .unwrap_or_else(|| "https://api.portaki.app".to_string());
    raw.trim_end_matches('/').to_string()
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
