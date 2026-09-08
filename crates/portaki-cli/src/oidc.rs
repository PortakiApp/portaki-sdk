//! Publier depuis une CI sans y stocker de secret.
//!
//! GitHub Actions donne à chaque job un jeton OIDC signé, valable le temps du job. Le registre
//! l'échange contre un droit de publier **un** module sur **un** canal, une fois. Rien à ranger
//! dans les secrets du dépôt, rien à faire tourner : le seul secret est celui que GitHub fabrique
//! et jette.
//!
//! Le jeton prouve d'où il vient, il n'autorise rien : c'est la liaison enregistrée chez le
//! registre — dépôt, workflow, environment, événement, runner — qui décide.

use anyhow::{bail, Context, Result};

/// Les deux variables que GitHub Actions pose quand le job demande `id-token: write`.
const REQUEST_URL: &str = "ACTIONS_ID_TOKEN_REQUEST_URL";
const REQUEST_TOKEN: &str = "ACTIONS_ID_TOKEN_REQUEST_TOKEN";

/// Tourne-t-on dans un job qui peut demander un jeton OIDC ?
///
/// L'absence de ces variables dans un job GitHub Actions veut presque toujours dire une chose :
/// `permissions: id-token: write` manque au workflow. Le message d'erreur le dit.
pub fn available() -> bool {
    non_empty(REQUEST_URL).is_some() && non_empty(REQUEST_TOKEN).is_some()
}

/// Sommes-nous chez GitHub Actions, jeton disponible ou non ?
pub fn inside_github_actions() -> bool {
    non_empty("GITHUB_ACTIONS").is_some()
}

fn non_empty(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// L'audience demandée pour le jeton.
///
/// Elle est choisie par l'appelant : elle lie le jeton à ce service, elle ne prouve rien sur
/// l'identité. Le registre la vérifie pour refuser un jeton émis pour ailleurs, et autorise sur
/// `repository_id`.
pub fn audience(base: &str) -> String {
    non_empty("PORTAKI_REGISTRY_OIDC_AUDIENCE")
        .unwrap_or_else(|| format!("{}/registry", base.trim_end_matches('/')))
}

/// Demande à GitHub un jeton pour cette audience.
pub async fn request_token(audience: &str) -> Result<String> {
    let url = non_empty(REQUEST_URL).context("ACTIONS_ID_TOKEN_REQUEST_URL absent")?;
    let bearer = non_empty(REQUEST_TOKEN).context("ACTIONS_ID_TOKEN_REQUEST_TOKEN absent")?;

    let response = reqwest::Client::new()
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

/// Échange le jeton OIDC contre un credential de publication.
///
/// Le credential est court et à usage unique : une publication qui échoue le consomme, et il
/// faut en redemander un. C'est sans conséquence — un job peut redemander un jeton OIDC autant
/// de fois qu'il veut.
pub async fn exchange(
    base: &str,
    module_id: &str,
    channel: &str,
    oidc_token: &str,
) -> Result<String> {
    let response = reqwest::Client::new()
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
        bail!("{}", refusal(status, &body));
    }
    let parsed: serde_json::Value =
        serde_json::from_str(&body).context("réponse d'échange illisible")?;
    match parsed.get("token").and_then(serde_json::Value::as_str) {
        Some(token) if !token.is_empty() => Ok(token.to_string()),
        _ => bail!("le registre n'a pas rendu de credential"),
    }
}

/// Le refus, traduit en ce qu'il y a à corriger.
///
/// Un `403` nu laisserait chercher entre cinq causes ; le registre renvoie un code stable pour
/// chacune, et c'est lui qu'on lit.
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
            "le canal stable exige l'environment déclaré dans la liaison : ajoute `environment:` au job"
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
    fn the_audience_defaults_to_the_registry_of_the_target_platform() {
        assert_eq!(
            audience("https://api.portaki.app/"),
            "https://api.portaki.app/registry"
        );
    }

    /// Chaque code de refus dit quoi corriger : sans ça, cinq causes pour un même 403.
    #[test]
    fn each_refusal_says_what_to_fix() {
        let refused = refusal(403, r#"{"code":"environment_required","message":"aucun"}"#);

        assert!(refused.contains("environment_required"));
        assert!(refused.contains("environment:"));
    }

    #[test]
    fn an_unknown_refusal_still_carries_the_body() {
        let refused = refusal(500, "<html>oops</html>");

        assert!(refused.contains("oops"));
    }
}
