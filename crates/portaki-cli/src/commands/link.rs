//! `portaki link` — lie le module à son dépôt, ou ouvre la page qui le fait.
//!
//! La première liaison exige de choisir une installation GitHub, ce qui ne se fait que dans le
//! dashboard : sans `--all`, le CLI ouvre la page Dépôt, avec les autres modules du monorepo en
//! `?also=`. Une fois le module courant lié, `--all` lie les autres modules du monorepo au même
//! dépôt, avec les mêmes règles, par `POST /dev/v1/module-links` — le registre refuse un nom pris
//! et le dit par module.

use anyhow::{Context, Result};
use clap::Parser;

use crate::commands::dev::{self, Unauthorized};
use crate::{auth, ui, workspace};

#[derive(Debug, Parser)]
/// Arguments for `portaki link`.
pub struct LinkArgs {
    /// In a repository holding several modules, the one whose page to open.
    #[arg(long)]
    pub module: Option<String>,
    /// Base URL of the platform. Defaults to PORTAKI_API_URL, then production.
    #[arg(long)]
    pub url: Option<String>,
    /// Print the link instead of opening it.
    #[arg(long)]
    pub no_browser: bool,
    /// Link every module of the monorepo to the repository this one is linked to, with its rules.
    #[arg(long, conflicts_with = "no_browser")]
    pub all: bool,
}

/// Runs `portaki link`.
pub async fn run(args: LinkArgs) -> Result<()> {
    ui::header(
        "portaki link",
        "Link the module to its repository — the first link is chosen in the dashboard.",
    );
    let current = workspace::resolve(args.module.as_deref(), None)?
        .into_iter()
        .next()
        .map(|member| member.id)
        .unwrap_or_default();
    if current.is_empty() {
        anyhow::bail!("portaki.module.json carries no id — run from the module root");
    }
    let cwd = std::env::current_dir()?;
    let others: Vec<String> = workspace::members(&cwd)
        .into_iter()
        .map(|member| member.id)
        .filter(|id| *id != current)
        .collect();

    if args.all {
        if others.is_empty() {
            ui::skipped("no other module in this repository — nothing to link");
            ui::blank();
            return Ok(());
        }
        if link_all(&args, &current, &others).await? {
            return Ok(());
        }
        ui::warn(format!(
            "{current} is not linked yet — link it in the dashboard first; --all then links the \
             others with the same rules"
        ));
    }

    let page = link_page(&auth::api_base_url(args.url.as_deref()), &current).await?;
    let target = with_also(&page, &others);
    if args.no_browser || !ui::open_browser(&target) {
        ui::field("open", &target);
    } else {
        ui::success("opened your browser");
        ui::field("link", &target);
    }
    ui::blank();
    Ok(())
}

/// Lie `others` au dépôt de `anchor`. `false` quand `anchor` n'est lié à rien : il n'y a alors
/// ni dépôt ni règles à reprendre.
async fn link_all(args: &LinkArgs, anchor: &str, others: &[String]) -> Result<bool> {
    let base = dev::resolve_base_url(
        args.url.as_deref(),
        std::env::var("PORTAKI_DEV_URL").ok().as_deref(),
        std::env::var("PORTAKI_API_URL").ok().as_deref(),
    );
    let mut token = auth::access_token()?;
    let reading = ui::step(format!("reading how {anchor} is linked"));
    let link = match read_link(&base, anchor, &token).await {
        Err(failure) if failure.is::<Unauthorized>() => {
            token = dev::renew(&auth::api_base_url(args.url.as_deref()), &token).await?;
            read_link(&base, anchor, &token).await
        }
        other => other,
    }
    .map_err(|failure| {
        reading.abandon();
        failure
    })?;
    let Some(link) = link else {
        reading.abandon();
        return Ok(false);
    };
    reading.done(format!(
        "{anchor} publishes from {}",
        link["repository"].as_str().unwrap_or("its repository")
    ));

    let linking = ui::step(format!("linking {} module(s)", others.len()));
    let response = crate::http::client()
        .post(format!("{base}/dev/v1/module-links"))
        .bearer_auth(&token)
        .json(&bulk_body(&link, others))
        .send()
        .await
        .context("ask the platform to link the modules")?;
    let status = response.status().as_u16();
    let body = response.text().await.unwrap_or_default();
    if !(200..300).contains(&status) {
        linking.abandon();
        anyhow::bail!(
            "the platform answered {status} to module-links: {}",
            body.trim()
        );
    }
    let (linked, refused) = outcomes(&body)?;
    linking.done(format!(
        "{} linked, {} refused",
        linked.len(),
        refused.len()
    ));
    for id in &linked {
        ui::success(id);
    }
    for (id, reason) in &refused {
        ui::failure(format!("{id} — {reason}"));
    }
    ui::blank();
    if !refused.is_empty() {
        anyhow::bail!("{} module(s) could not be linked", refused.len());
    }
    Ok(true)
}

/// La liaison d'un module, `None` s'il n'en a pas.
async fn read_link(base: &str, module_id: &str, token: &str) -> Result<Option<serde_json::Value>> {
    let response = crate::http::client()
        .get(format!("{base}/dev/v1/modules/{module_id}/link"))
        .bearer_auth(token)
        .send()
        .await
        .context("read the module's link")?;
    match response.status().as_u16() {
        401 => Err(anyhow::Error::new(Unauthorized)),
        404 => Ok(None),
        _ => dev::read_json(response).await.map(Some),
    }
}

/// `{ repositoryId, moduleIds, rules }`, les règles étant celles du module déjà lié.
///
/// Les règles sont aussi posées à plat : c'est la forme que le registre lisait avant `rules`, et
/// un CLI plus récent que la plateforme ne doit pas se faire refuser pour autant.
fn bulk_body(link: &serde_json::Value, others: &[String]) -> serde_json::Value {
    let mut rules = serde_json::Map::new();
    for key in [
        "installationId",
        "expectedWorkflow",
        "requiredEnvironment",
        "allowedEvents",
        "githubHostedOnly",
    ] {
        if let Some(value) = link.get(key) {
            rules.insert(key.to_string(), value.clone());
        }
    }
    let mut body = rules.clone();
    body.insert("repositoryId".into(), link["repositoryId"].clone());
    body.insert("moduleIds".into(), others.into());
    body.insert("rules".into(), rules.into());
    body.into()
}

/// Un module refusé, et pourquoi.
type Refused = (String, String);

/// Les modules liés, et les refusés avec leur raison.
///
/// `{ linked, refused: [{ moduleId, reason }] }`, ou la liste `[{ moduleId, linked, code,
/// message }]` d'un registre antérieur.
fn outcomes(body: &str) -> Result<(Vec<String>, Vec<Refused>)> {
    #[derive(serde::Deserialize)]
    #[serde(untagged)]
    enum Answer {
        Split {
            #[serde(default)]
            linked: Vec<String>,
            #[serde(default)]
            refused: Vec<Refusal>,
        },
        Each(Vec<Outcome>),
    }
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Refusal {
        module_id: String,
        #[serde(default)]
        reason: String,
    }
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Outcome {
        module_id: String,
        linked: bool,
        #[serde(default)]
        code: Option<String>,
        #[serde(default)]
        message: Option<String>,
    }

    let answer: Answer =
        serde_json::from_str(body).with_context(|| format!("unexpected answer: {body}"))?;
    Ok(match answer {
        Answer::Split { linked, refused } => (
            linked,
            refused
                .into_iter()
                .map(|r| (r.module_id, r.reason))
                .collect(),
        ),
        Answer::Each(each) => {
            let (linked, refused): (Vec<_>, Vec<_>) = each.into_iter().partition(|o| o.linked);
            (
                linked.into_iter().map(|o| o.module_id).collect(),
                refused
                    .into_iter()
                    .map(|o| (o.module_id, o.message.or(o.code).unwrap_or_default()))
                    .collect(),
            )
        }
    })
}

/// Ajoute les autres modules à la page Dépôt rendue par le registre, en `?also=`.
///
/// L'adresse de la page vient de l'API (`link-page`, ou `linkUrl` d'un refus) : le CLI ne la
/// déduit plus du nom de l'API, une règle de nommage qui casse au premier environnement qui ne
/// la suit pas.
pub fn with_also(page: &str, others: &[String]) -> String {
    if others.is_empty() {
        return page.to_string();
    }
    let separator = if page.contains('?') { '&' } else { '?' };
    format!("{page}{separator}also={}", others.join(","))
}

/// La page Dépôt d'un module, telle que le registre la donne.
async fn link_page(api_base: &str, module_id: &str) -> Result<String> {
    let url = format!(
        "{}/registry/v1/modules/{module_id}/link-page",
        api_base.trim_end_matches('/')
    );
    let response = crate::http::client()
        .get(&url)
        .send()
        .await
        .with_context(|| format!("demander la page Dépôt au registre ({url})"))?;
    let status = response.status().as_u16();
    let body = response.text().await.unwrap_or_default();
    if !(200..300).contains(&status) {
        anyhow::bail!("le registre n'a pas rendu la page Dépôt ({status}) : {body}");
    }
    serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|parsed| parsed.get("url")?.as_str().map(str::to_string))
        .context("réponse link-page sans url")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(raw: &[&str]) -> Vec<String> {
        raw.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn the_bulk_link_carries_the_anchor_rules() {
        let link = serde_json::json!({
            "moduleId": "access-guide", "installationId": 7, "repositoryId": 42,
            "repository": "acme/modules", "expectedWorkflow": "release.yml",
            "requiredEnvironment": null, "allowedEvents": ["push"], "githubHostedOnly": true,
            "linkedAt": "2026-09-01T00:00:00Z"
        });

        let body = bulk_body(&link, &ids(&["nuki"]));

        assert_eq!(body["repositoryId"], 42);
        assert_eq!(body["moduleIds"], serde_json::json!(["nuki"]));
        assert_eq!(body["rules"]["expectedWorkflow"], "release.yml");
        assert_eq!(body["rules"]["installationId"], 7);
        assert!(body["rules"].get("moduleId").is_none());
        assert_eq!(body["expectedWorkflow"], "release.yml");
    }

    #[test]
    fn both_answer_shapes_are_read() {
        let (linked, refused) =
            outcomes(r#"{"linked":["nuki"],"refused":[{"moduleId":"weather","reason":"taken"}]}"#)
                .unwrap();
        assert_eq!(linked, ids(&["nuki"]));
        assert_eq!(refused, vec![("weather".to_string(), "taken".to_string())]);

        let (linked, refused) = outcomes(
            r#"[{"moduleId":"nuki","linked":true},{"moduleId":"weather","linked":false,"code":"module_taken","message":"owned by someone else"}]"#,
        )
        .unwrap();
        assert_eq!(linked, ids(&["nuki"]));
        assert_eq!(refused[0].1, "owned by someone else");
    }

    #[test]
    fn the_other_modules_ride_along_in_also() {
        assert_eq!(
            with_also(
                "https://developer.portaki.app/access-guide/repository",
                &ids(&["nuki", "wifi-guest"])
            ),
            "https://developer.portaki.app/access-guide/repository?also=nuki,wifi-guest"
        );
        assert_eq!(
            with_also("https://developer.portaki.app/weather/repository", &[]),
            "https://developer.portaki.app/weather/repository"
        );
        assert_eq!(
            with_also(
                "http://localhost:3000/dev/x/repository?from=cli",
                &ids(&["y"])
            ),
            "http://localhost:3000/dev/x/repository?from=cli&also=y"
        );
    }
}
