//! The envelope the platform wraps around every `/api/v1` response.
//!
//! Every answer from the orchestrator arrives in the form `{"success": …, "data": …}`, and an
//! error adds `error_code`. This is not decoration: reading the body directly gives an object in
//! which none of the expected fields exist, and the failure then looks like a network problem
//! when it is really one of shape.
//!
//! devapi does not go through this — its routes are under `/dev/v1` and answer the bare object.

use anyhow::{bail, Result};
use serde::de::DeserializeOwned;

#[derive(Debug, serde::Deserialize)]
struct Envelope<T> {
    #[serde(default)]
    success: bool,
    // An explicit `default` rather than a derived one: the derived one would require `T: Default`
    // of every type carried, whereas the absence of `data` is already represented by `None`.
    #[serde(default = "no_data")]
    data: Option<T>,
    #[serde(default)]
    error_code: Option<String>,
}

fn no_data<T>() -> Option<T> {
    None
}

/// Pulls the useful body out of a response announced as successful.
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

/// The error code, when the response carries one. Absent, the caller decides what to say about it.
///
/// The `/api/v1` envelope carries it as `error_code`; devapi and the registry, as `code`.
pub fn error_code(body: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    ["error_code", "code"]
        .iter()
        .find_map(|key| value[key].as_str().map(str::to_string))
}

/// A session with the platform, to read its bare JSON routes (`/dev/v1`, `/registry/v1`).
///
/// A 401 renews the token once; a 404 reads as "nothing", not as "breakdown".
pub struct Platform {
    pub base: String,
    token: String,
}

impl Platform {
    /// The session for `base`, or [`crate::auth::NotSignedIn`].
    pub fn open(base: &str) -> Result<Self> {
        Ok(Self {
            base: base.trim_end_matches('/').to_string(),
            token: crate::auth::access_token(base)?,
        })
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    /// `GET {base}{path}`: the body, `None` on a 404.
    pub async fn get(&mut self, path: &str) -> Result<Option<serde_json::Value>> {
        let (status, body) = self.send(reqwest::Method::GET, path, None).await?;
        if status == 404 {
            return Ok(None);
        }
        self.accept(path, status, body).map(Some)
    }

    /// `POST`, `PUT`, `PATCH`: the body of the response, or the refusal — a 404 included, since
    /// we were writing something.
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

    /// The request, renewed once on a 401: the status and the body read as JSON (`Null` when it
    /// is empty, the raw string when it is not JSON).
    ///
    /// Patient outside `GET`: a dispatch, a render or a grid of scenarios run the module before
    /// answering.
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

    /// The defect this module fixes: read flat, this body gives none of the expected fields.
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
