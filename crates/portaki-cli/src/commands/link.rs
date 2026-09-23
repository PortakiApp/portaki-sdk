//! `portaki link` — ouvre la page Dépôt du module dans la console développeur.
//!
//! Le CLI ne crée jamais la liaison module ↔ dépôt : elle exige de choisir une installation
//! GitHub, ce qui ne se fait que dans le dashboard. Il ouvre la page, avec les autres modules du
//! monorepo en `?also=` — le dashboard ignore de lui-même ceux qui sont déjà liés ou inconnus.

use anyhow::Result;
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
pub fn run(args: LinkArgs) -> Result<()> {
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
    let mut ids = vec![current.clone()];
    ids.extend(
        workspace::members(&cwd)
            .into_iter()
            .map(|member| member.id)
            .filter(|id| *id != current),
    );

    let target = repository_url(
        &developer_origin(&auth::api_base_url(args.url.as_deref())),
        &ids,
    );
    if args.no_browser || !ui::open_browser(&target) {
        ui::field("open", &target);
    } else {
        ui::success("opened your browser");
        ui::field("link", &target);
    }
    ui::blank();
    Ok(())
}

/// La page Dépôt du premier module, les suivants en `?also=`.
///
/// Sur `developer.<racine>` les chemins n'ont pas de préfixe `/dev` : le proxy du dashboard
/// réécrit `/x` en `/dev/x`, et redirige `/dev/x` vers `/x`.
pub fn repository_url(origin: &str, ids: &[String]) -> String {
    let (first, others) = ids.split_first().map_or(("", &[][..]), |(f, o)| (f, o));
    let mut url = format!("{}/{first}/repository", origin.trim_end_matches('/'));
    if !others.is_empty() {
        url.push_str("?also=");
        url.push_str(&others.join(","));
    }
    url
}

/// L'origine de la console développeur, déduite de l'API visée.
///
/// `api.<racine>` → `developer.<racine>`, `api-staging.portaki.app` →
/// `developer.staging.portaki.app` : la même racine que le dashboard (`getRootDomain`). En local,
/// la console est servie sous `/dev` du dashboard. `PORTAKI_DEVELOPER_URL` tranche sinon.
pub fn developer_origin(api_base: &str) -> String {
    if let Some(explicit) = std::env::var("PORTAKI_DEVELOPER_URL")
        .ok()
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty())
    {
        return explicit;
    }
    derive_origin(api_base)
}

/// La règle seule, sans l'environnement, pour être vérifiable.
fn derive_origin(api_base: &str) -> String {
    const PRODUCTION: &str = "https://developer.portaki.app";
    let Some(host) = reqwest::Url::parse(api_base)
        .ok()
        .and_then(|url| url.host_str().map(str::to_string))
    else {
        return PRODUCTION.to_string();
    };
    if host == "localhost" || host == "127.0.0.1" {
        return "http://localhost:3000/dev".to_string();
    }
    if let Some(root) = host.strip_prefix("api.") {
        return format!("https://developer.{root}");
    }
    if let Some((env, root)) = host
        .strip_prefix("api-")
        .and_then(|rest| rest.split_once('.'))
    {
        return format!("https://developer.{env}.{root}");
    }
    PRODUCTION.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(raw: &[&str]) -> Vec<String> {
        raw.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn the_link_names_the_first_module_and_carries_the_others() {
        assert_eq!(
            repository_url(
                "https://developer.portaki.app",
                &ids(&["access-guide", "nuki", "wifi-guest"])
            ),
            "https://developer.portaki.app/access-guide/repository?also=nuki,wifi-guest"
        );
        assert_eq!(
            repository_url("https://developer.portaki.app/", &ids(&["weather"])),
            "https://developer.portaki.app/weather/repository"
        );
    }

    #[test]
    fn the_console_follows_the_platform_it_talks_to() {
        assert_eq!(
            derive_origin("https://api.portaki.app"),
            "https://developer.portaki.app"
        );
        assert_eq!(
            derive_origin("https://api-staging.portaki.app"),
            "https://developer.staging.portaki.app"
        );
        assert_eq!(
            derive_origin("http://localhost:8080"),
            "http://localhost:3000/dev"
        );
        assert_eq!(derive_origin("nope"), "https://developer.portaki.app");
    }
}
