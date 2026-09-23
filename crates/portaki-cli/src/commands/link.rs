//! `portaki link` — ouvre la page Dépôt du module dans la console développeur.
//!
//! Le CLI ne crée jamais la liaison module ↔ dépôt : elle exige de choisir une installation
//! GitHub, ce qui ne se fait que dans le dashboard. Il ouvre la page, avec les autres modules du
//! monorepo en `?also=` — le dashboard ignore de lui-même ceux qui sont déjà liés ou inconnus.

use anyhow::{Context, Result};
use clap::Parser;

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
}

/// Runs `portaki link`.
pub async fn run(args: LinkArgs) -> Result<()> {
    ui::header(
        "portaki link",
        "Open the repository page — linking needs a GitHub installation, chosen in the dashboard.",
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
