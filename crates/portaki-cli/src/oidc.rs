//! Publishing from a CI without storing a secret there.
//!
//! GitHub Actions gives every job a signed OIDC token, valid for the length of the job. The
//! registry exchanges it for the right to publish **one** module on **one** channel, once.
//! Nothing to keep in the repository's secrets, nothing to rotate: the only secret is the one
//! GitHub makes and throws away.
//!
//! The token proves where it comes from, it authorises nothing: it is the link registered with
//! the registry — repository, workflow, environment, event, runner — that decides.

use anyhow::{bail, Context, Result};

/// The two variables GitHub Actions sets when the job asks for `id-token: write`.
const REQUEST_URL: &str = "ACTIONS_ID_TOKEN_REQUEST_URL";
const REQUEST_TOKEN: &str = "ACTIONS_ID_TOKEN_REQUEST_TOKEN";

/// Are we running inside a job that can ask for an OIDC token?
///
/// The absence of these variables in a GitHub Actions job almost always means one thing:
/// `permissions: id-token: write` is missing from the workflow. The error message says so.
pub fn available() -> bool {
    non_empty(REQUEST_URL).is_some() && non_empty(REQUEST_TOKEN).is_some()
}

/// Are we on GitHub Actions, token available or not?
pub fn inside_github_actions() -> bool {
    non_empty("GITHUB_ACTIONS").is_some()
}

fn non_empty(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// The audience requested for the token.
///
/// It is chosen by the caller: it binds the token to this service, it proves nothing about
/// identity. The registry checks it in order to refuse a token issued for somewhere else, and
/// authorises on `repository_id`.
pub fn audience(base: &str) -> String {
    non_empty("PORTAKI_REGISTRY_OIDC_AUDIENCE")
        .unwrap_or_else(|| format!("{}/registry", base.trim_end_matches('/')))
}

/// Asks GitHub for a token for this audience.
pub async fn request_token(audience: &str) -> Result<String> {
    let url = non_empty(REQUEST_URL).context("ACTIONS_ID_TOKEN_REQUEST_URL absent")?;
    let bearer = non_empty(REQUEST_TOKEN).context("ACTIONS_ID_TOKEN_REQUEST_TOKEN absent")?;

    let response = crate::http::client()
        .get(url)
        .query(&[("audience", audience)])
        .header("Authorization", format!("bearer {bearer}"))
        .send()
        .await
        .context("demander un jeton OIDC à GitHub Actions")?;

    let status = response.status().as_u16();
    let body = response.text().await.unwrap_or_default();
    if !(200..300).contains(&status) {
        bail!("GitHub a refusé d'émettre un jeton OIDC ({status}) : {body}");
    }
    let parsed: serde_json::Value =
        serde_json::from_str(&body).context("réponse OIDC illisible")?;
    match parsed.get("value").and_then(serde_json::Value::as_str) {
        Some(token) if !token.is_empty() => Ok(token.to_string()),
        _ => bail!("la réponse OIDC de GitHub ne porte pas de jeton"),
    }
}

/// Exchanges the OIDC token for a publication credential.
///
/// The credential is short-lived and single-use: a publication that fails consumes it, and
/// another one has to be asked for. That is of no consequence — a job can ask for an OIDC token
/// as many times as it likes.
pub async fn exchange(
    base: &str,
    module_id: &str,
    channel: &str,
    oidc_token: &str,
) -> Result<String> {
    let response = crate::http::client()
        .post(format!(
            "{}/registry/v1/publications/token",
            base.trim_end_matches('/')
        ))
        .bearer_auth(oidc_token)
        .json(&serde_json::json!({ "moduleId": module_id, "channel": channel }))
        .send()
        .await
        .context("échanger le jeton OIDC contre un credential de publication")?;

    let status = response.status().as_u16();
    let body = response.text().await.unwrap_or_default();
    if !(200..300).contains(&status) {
        return Err(Refused::from_response(status, &body).into());
    }
    let parsed: serde_json::Value =
        serde_json::from_str(&body).context("réponse d'échange illisible")?;
    match parsed.get("token").and_then(serde_json::Value::as_str) {
        Some(token) if !token.is_empty() => Ok(token.to_string()),
        _ => bail!("le registre n'a pas rendu de credential"),
    }
}

/// A refused exchange, with the registry's stable code: `publish` reads it to recognise
/// `module_not_linked` without parsing a message.
#[derive(Debug)]
pub struct Refused {
    pub status: u16,
    pub code: String,
    /// The page where the refusal is to be fixed, when the registry gives one
    /// (`module_not_linked`).
    pub link_url: Option<String>,
    text: String,
}

impl Refused {
    pub(crate) fn from_response(status: u16, body: &str) -> Self {
        let parsed = serde_json::from_str::<serde_json::Value>(body).ok();
        let field = |name: &str| {
            parsed
                .as_ref()
                .and_then(|parsed| parsed.get(name)?.as_str().map(str::to_string))
        };
        Self {
            status,
            code: field("code").unwrap_or_default(),
            link_url: field("linkUrl"),
            text: refusal(status, body),
        }
    }
}

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.text)
    }
}

impl std::error::Error for Refused {}

/// The refusal, translated into what there is to fix.
///
/// A bare `403` would leave you searching among five causes; the registry returns a stable code
/// for each one, and that is what we read.
fn refusal(status: u16, body: &str) -> String {
    let parsed: serde_json::Value = serde_json::from_str(body).unwrap_or(serde_json::Value::Null);
    let code = parsed
        .get("code")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let message = parsed
        .get("message")
        .and_then(serde_json::Value::as_str)
        .unwrap_or(body);
    let hint = match code {
        "module_not_linked" => {
            "lie ce module à ce dépôt depuis le dashboard avant de publier depuis la CI"
        }
        "repository_not_linked" => "ce module est lié à un autre dépôt",
        "workflow_not_allowed" => {
            "la liaison attend un autre fichier de workflow — corrige-la ou publie depuis celui déclaré"
        }
        "environment_required" => {
            "le canal stable exige l'environnement GitHub `release` : ajoutez `environment: release` au job de publication, et nommez-le dans la liaison"
        }
        "event_not_allowed" => "cet événement déclencheur n'est pas autorisé par la liaison",
        "runner_not_allowed" => "la liaison exige un runner hébergé par GitHub",
        "token_replayed" => "ce jeton OIDC a déjà été échangé — demandes-en un nouveau",
        "github_oidc_required" => {
            "le registre n'a pas reconnu le jeton : vérifie l'audience (PORTAKI_REGISTRY_OIDC_AUDIENCE)"
        }
        _ => "",
    };
    if hint.is_empty() {
        format!("le registre a refusé l'échange ({status} {code}) : {message}")
    } else {
        format!("le registre a refusé l'échange ({status} {code}) : {message} — {hint}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_not_linked_refusal_carries_the_page_the_registry_gives() {
        let refused = Refused::from_response(
            403,
            r#"{"code":"module_not_linked","message":"x","linkUrl":"https://developer.portaki.app/nuki/repository"}"#,
        );

        assert_eq!(
            refused.link_url.as_deref(),
            Some("https://developer.portaki.app/nuki/repository")
        );
    }

    #[test]
    fn the_audience_defaults_to_the_registry_of_the_target_platform() {
        assert_eq!(
            audience("https://api.portaki.app/"),
            "https://api.portaki.app/registry"
        );
    }

    /// Every refusal code says what to fix: without that, five causes behind one same 403.
    #[test]
    fn each_refusal_says_what_to_fix() {
        let refused = refusal(403, r#"{"code":"environment_required","message":"aucun"}"#);

        assert!(refused.contains("environment_required"));
        assert!(refused.contains("environment: release"));
    }

    #[test]
    fn a_refusal_keeps_its_code_for_the_caller() {
        let refused = Refused::from_response(403, r#"{"code":"module_not_linked","message":"x"}"#);

        assert_eq!(refused.code, "module_not_linked");
        assert_eq!(refused.link_url, None);
        assert!(refused.to_string().contains("module_not_linked"));
    }

    #[test]
    fn an_unknown_refusal_still_carries_the_body() {
        let refused = refusal(500, "<html>oops</html>");

        assert!(refused.contains("oops"));
    }
}
