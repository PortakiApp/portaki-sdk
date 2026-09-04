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
pub fn error_code(body: &str) -> Option<String> {
    serde_json::from_str::<Envelope<serde_json::Value>>(body)
        .ok()
        .and_then(|envelope| envelope.error_code)
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
