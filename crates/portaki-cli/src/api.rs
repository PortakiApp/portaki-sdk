//! L'enveloppe que la plateforme met autour de chaque réponse `/api/v1`.
//!
//! Toute réponse de l'orchestrator arrive sous la forme `{"success": …, "data": …}`, et une
//! erreur ajoute `error_code`. Ce n'est pas une décoration : lire directement le corps donne un
//! objet dont aucun champ attendu n'existe, et l'échec ressemble alors à un problème de réseau
//! alors qu'il est de forme.
//!
//! devapi ne passe pas par là — ses routes sont sous `/dev/v1` et répondent l'objet nu.

use anyhow::{bail, Result};
use serde::de::DeserializeOwned;

#[derive(Debug, serde::Deserialize)]
struct Envelope<T> {
    #[serde(default)]
    success: bool,
    // `default` explicite plutôt que dérivé : le dérivé exigerait `T: Default` de chaque type
    // transporté, alors que l'absence de `data` se représente déjà par `None`.
    #[serde(default = "no_data")]
    data: Option<T>,
    #[serde(default)]
    error_code: Option<String>,
}

fn no_data<T>() -> Option<T> {
    None
}

/// Sort le corps utile d'une réponse annoncée comme réussie.
pub fn unwrap<T: DeserializeOwned>(body: &str) -> Result<T> {
    let envelope: Envelope<T> = serde_json::from_str(body)
        .map_err(|failure| anyhow::anyhow!("unexpected answer ({failure}): {body}"))?;
    match envelope.data {
        Some(data) if envelope.success => Ok(data),
        _ => bail!(
            "the platform answered {}: {body}",
            envelope.error_code.unwrap_or_else(|| "no data".to_string())
        ),
    }
}

/// Le code d'erreur, quand la réponse en porte un. Absent, l'appelant décide quoi en dire.
///
/// L'enveloppe `/api/v1` le porte en `error_code` ; devapi et le registre, en `code`.
pub fn error_code(body: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    ["error_code", "code"]
        .iter()
        .find_map(|key| value[key].as_str().map(str::to_string))
}

/// Une session auprès de la plateforme, pour lire ses routes JSON nues (`/dev/v1`, `/registry/v1`).
///
/// Un 401 renouvelle le jeton une fois ; un 404 se lit « rien », pas « panne ».
pub struct Platform {
    pub base: String,
    token: String,
}

impl Platform {
    /// La session de `base`, ou [`crate::auth::NotSignedIn`].
    pub fn open(base: &str) -> Result<Self> {
        Ok(Self {
            base: base.trim_end_matches('/').to_string(),
            token: crate::auth::access_token(base)?,
        })
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    /// `GET {base}{path}` : le corps, `None` sur un 404.
    pub async fn get(&mut self, path: &str) -> Result<Option<serde_json::Value>> {
        let (status, body) = self.send(reqwest::Method::GET, path, None).await?;
        if status == 404 {
            return Ok(None);
        }
        self.accept(path, status, body).map(Some)
    }

    /// `POST`, `PUT`, `PATCH` : le corps de la réponse, ou le refus — un 404 compris, puisqu'on
    /// écrivait quelque chose.
    pub async fn call(
        &mut self,
        method: reqwest::Method,
        path: &str,
        body: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let (status, answer) = self.send(method, path, body).await?;
        self.accept(path, status, answer)
    }

    fn accept(
        &self,
        path: &str,
        status: u16,
        body: serde_json::Value,
    ) -> Result<serde_json::Value> {
        if (200..300).contains(&status) {
            return Ok(body);
        }
        let url = format!("{}{path}", self.base);
        let raw = body.to_string();
        let refused = crate::http::refused(&url, status, &raw);
        match body["message"]
            .as_str()
            .filter(|message| !message.is_empty())
        {
            Some(message) => bail!("{refused} — {message}"),
            None => bail!("{refused}"),
        }
    }

    /// La requête, renouvelée une fois sur un 401 : le statut et le corps lu en JSON (`Null`
    /// quand il est vide, la chaîne brute quand il n'est pas du JSON).
    ///
    /// Patient hors `GET` : un dispatch, un rendu ou une grille de scénarios exécutent le module
    /// avant de répondre.
    pub async fn send(
        &mut self,
        method: reqwest::Method,
        path: &str,
        body: Option<&serde_json::Value>,
    ) -> Result<(u16, serde_json::Value)> {
        use anyhow::Context as _;
        let url = format!("{}{path}", self.base);
        let client = if method == reqwest::Method::GET {
            crate::http::client()
        } else {
            crate::http::patient_client()
        };
        let mut renewed = false;
        loop {
            let mut request = client
                .request(method.clone(), &url)
                .bearer_auth(&self.token);
            if let Some(body) = body {
                request = request.json(body);
            }
            let response = request
                .send()
                .await
                .map_err(|failure| crate::http::unreachable(&url, failure))?;
            let status = response.status().as_u16();
            if status == 401 && !renewed {
                renewed = true;
                self.token = crate::auth::refresh(&self.base, &self.token)
                    .await
                    .context("renew the session")?;
                continue;
            }
            let raw = response.text().await.unwrap_or_default();
            let parsed = if raw.trim().is_empty() {
                serde_json::Value::Null
            } else {
                serde_json::from_str(&raw).unwrap_or(serde_json::Value::String(raw))
            };
            return Ok((status, parsed));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, serde::Deserialize, PartialEq)]
    #[serde(rename_all = "camelCase")]
    struct Token {
        access_token: String,
    }

    #[test]
    fn the_useful_body_lives_under_data() {
        let parsed: Token =
            unwrap(r#"{"success":true,"data":{"accessToken":"abc"}}"#).expect("unwrap");

        assert_eq!(parsed.access_token, "abc");
    }

    /// Le défaut que ce module corrige : lu à plat, ce corps ne donne aucun champ attendu.
    #[test]
    fn a_flat_read_of_the_same_body_would_have_failed() {
        assert!(
            serde_json::from_str::<Token>(r#"{"success":true,"data":{"accessToken":"abc"}}"#)
                .is_err()
        );
    }

    #[test]
    fn an_error_body_names_its_code_instead_of_pretending_to_be_data() {
        let body = r#"{"success":false,"error_code":"authorization_pending"}"#;

        assert_eq!(error_code(body).as_deref(), Some("authorization_pending"));
        assert!(unwrap::<Token>(body)
            .unwrap_err()
            .to_string()
            .contains("authorization_pending"));
    }

    #[test]
    fn a_body_that_is_not_an_envelope_reports_the_body_itself() {
        let failure = unwrap::<Token>("<html>502</html>").unwrap_err().to_string();

        assert!(failure.contains("502"), "{failure}");
        assert!(error_code("<html>502</html>").is_none());
    }
}
