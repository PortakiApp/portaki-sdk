//! `portaki link` — links the module to its repository, or opens the page that does it.
//!
//! The first link requires choosing a GitHub installation, which can only be done in the
//! dashboard: without `--all`, the CLI opens the Repository page, with the monorepo's other
//! modules in `?also=`. Once the current module is linked, `--all` links the monorepo's other
//! modules to the same repository, with the same rules, through `POST /dev/v1/module-links` — the
//! registry refuses a name already taken, and says so module by module.

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
    /// Alias of the global --api, kept for older scripts.
    #[arg(long, hide = true)]
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
        &crate::tr!("Link the module to its repository — the first link is chosen in the dashboard.", "Lier le module à son dépôt — la première liaison se choisit dans l'espace développeur."),
    );
    let current = workspace::resolve(args.module.as_deref(), None)?
        .into_iter()
        .next()
        .map(|member| member.id)
        .unwrap_or_default();
    if current.is_empty() {
        anyhow::bail!(
            "no module here — its id comes from [package] name in Cargo.toml; run from the \
             module root, or pass --module <id>"
        );
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

    let page = link_page(&crate::profile::api_url(args.url.as_deref()), &current).await?;
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

/// Links `others` to `anchor`'s repository. `false` when `anchor` is linked to nothing: there is
/// then neither a repository nor rules to carry over.
async fn link_all(args: &LinkArgs, anchor: &str, others: &[String]) -> Result<bool> {
    let base = crate::profile::api_url(args.url.as_deref());
    let mut token = auth::access_token(&base)?;
    let reading = ui::step(format!("reading how {anchor} is linked"));
    let link = match read_link(&base, anchor, &token).await {
        Err(failure) if failure.is::<Unauthorized>() => {
            token = dev::renew(&crate::profile::api_url(args.url.as_deref()), &token).await?;
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

/// A module's link, `None` when it has none.
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

/// `{ repositoryId, moduleIds, rules }`, the rules being those of the module already linked.
///
/// The rules are laid out flat as well: that is the shape the registry read before `rules`, and a
/// CLI newer than the platform must not get refused just for that.
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

/// A refused module, and why.
type Refused = (String, String);

/// The linked modules, and the refused ones with their reason.
///
/// `{ linked, refused: [{ moduleId, reason }] }`, or the `[{ moduleId, linked, code, message }]`
/// list of an earlier registry.
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

/// Adds the other modules to the Repository page the registry serves, as `?also=`.
///
/// The page's address comes from the API (`link-page`, or the `linkUrl` of a refusal): the CLI no
/// longer derives it from the API's name, a naming rule that breaks on the first environment that
/// does not follow it.
pub fn with_also(page: &str, others: &[String]) -> String {
    if others.is_empty() {
        return page.to_string();
    }
    let separator = if page.contains('?') { '&' } else { '?' };
    format!("{page}{separator}also={}", others.join(","))
}

/// A module's Repository page, as the registry gives it.
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
