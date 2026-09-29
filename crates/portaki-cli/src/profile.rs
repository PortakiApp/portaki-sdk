//! Where the CLI talks: a single API base, chosen once for the whole command.
//!
//! `dev`, `logs` and `sdk upgrade` used to read `PORTAKI_DEV_URL` before `PORTAKI_API_URL`, and
//! `login` and `publish` the other way round: two variables drifting apart sent a command to one
//! platform with another one's session ("the stored session belongs to…"). There is only one
//! base left.
//!
//! In order: a command's `--url` (hidden, legacy alias), `--api`, `--env`, `PORTAKI_API_URL`,
//! `PORTAKI_DEV_URL` (deprecated), production.
//!
//! Profiles: `prod`, `staging` and `local` are known; `~/.config/portaki/config.toml` adds to
//! them or redefines them:
//!
//! ```toml
//! [env.preprod]
//! api = "https://api-preprod.example"
//! ```

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

use anyhow::{Context, Result};

/// The production platform.
pub const PRODUCTION: &str = "https://api.portaki.app";

const BUILT_IN: [(&str, &str); 3] = [
    ("prod", PRODUCTION),
    ("staging", "https://api-staging.portaki.app"),
    ("local", "http://localhost:8080"),
];

/// `--api`, or the URL of the `--env` profile, fixed once at startup.
static CHOSEN: OnceLock<Option<String>> = OnceLock::new();

/// The warning about `PORTAKI_DEV_URL` is not repeated on every call.
static WARNED: AtomicBool = AtomicBool::new(false);

/// Remembers `--api` / `--env`. An unknown profile is a usage error: going to production in
/// place of a mistyped `--env stagging` would be the worst possible reading.
pub fn select(api: Option<&str>, env: Option<&str>) -> Result<()> {
    let chosen = match (non_blank(api), non_blank(env)) {
        (Some(api), _) => Some(api.to_string()),
        (None, Some(name)) => Some(profile_url(name, &profiles()?)?),
        (None, None) => None,
    };
    let _ = CHOSEN.set(chosen);
    Ok(())
}

/// This command's API base, with no trailing slash.
pub fn api_url(explicit: Option<&str>) -> String {
    let dev = std::env::var("PORTAKI_DEV_URL").ok();
    if non_blank(dev.as_deref()).is_some() && !WARNED.swap(true, Ordering::Relaxed) {
        crate::ui::warn(
            "PORTAKI_DEV_URL is deprecated — set PORTAKI_API_URL (or --api / --env) instead",
        );
    }
    resolve(
        explicit,
        CHOSEN.get().and_then(Option::as_deref),
        std::env::var("PORTAKI_API_URL").ok().as_deref(),
        dev.as_deref(),
    )
}

/// The rule alone, without the environment, so that it can be checked.
///
/// An empty or blank value counts as "unset": a CI action passing an optional input that was
/// left unfilled exports an empty variable.
fn resolve(
    explicit: Option<&str>,
    chosen: Option<&str>,
    api_var: Option<&str>,
    dev_var: Option<&str>,
) -> String {
    [explicit, chosen, api_var, dev_var]
        .into_iter()
        .find_map(non_blank)
        .unwrap_or(PRODUCTION)
        .trim_end_matches('/')
        .to_string()
}

fn non_blank(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

/// The known profiles: the CLI's own, then the file's, which win.
pub fn profiles() -> Result<Vec<(String, String)>> {
    let path = crate::auth::config_dir()?.join("config.toml");
    let raw = match std::fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(missing) if missing.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(failure) => return Err(failure).with_context(|| format!("read {}", path.display())),
    };
    parse_profiles(&raw).with_context(|| format!("read {}", path.display()))
}

fn parse_profiles(raw: &str) -> Result<Vec<(String, String)>> {
    let mut found: Vec<(String, String)> = BUILT_IN
        .iter()
        .map(|(name, url)| (name.to_string(), url.to_string()))
        .collect();
    let document: toml_edit::DocumentMut = raw.parse().context("not valid TOML")?;
    let Some(envs) = document.get("env").and_then(toml_edit::Item::as_table_like) else {
        return Ok(found);
    };
    for (name, table) in envs.iter() {
        let url = table
            .get("api")
            .and_then(toml_edit::Item::as_str)
            .with_context(|| format!("[env.{name}] needs api = \"https://…\""))?;
        found.retain(|(known, _)| known != name);
        found.push((name.to_string(), url.trim_end_matches('/').to_string()));
    }
    Ok(found)
}

fn profile_url(name: &str, profiles: &[(String, String)]) -> Result<String> {
    profiles
        .iter()
        .find(|(known, _)| known == name)
        .map(|(_, url)| url.clone())
        .ok_or_else(|| {
            let names: Vec<&str> = profiles.iter().map(|(known, _)| known.as_str()).collect();
            crate::exit::usage(format!(
                "unknown environment: {name} — known: {} (add one under [env.{name}] in ~/.config/portaki/config.toml)",
                names.join(", ")
            ))
        })
}

/// The command that opens a session on `url` — what a command without a session has to say.
pub fn login_command(url: &str) -> String {
    let origin = crate::auth::origin_of(url);
    let named = profiles().ok().and_then(|profiles| {
        profiles
            .into_iter()
            .find(|(_, known)| crate::auth::origin_of(known) == origin)
            .map(|(name, _)| name)
    });
    match named.as_deref() {
        Some("prod") => "portaki login".to_string(),
        Some(name) => format!("portaki login --env {name}"),
        None => format!("portaki login --api {}", url.trim_end_matches('/')),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn precedence_is_flag_then_profile_then_variables_then_production() {
        let staging = "https://api-staging.portaki.app";
        assert_eq!(
            resolve(Some("https://x.example/"), Some(staging), None, None),
            "https://x.example"
        );
        assert_eq!(
            resolve(None, Some(staging), Some("https://env.example"), None),
            staging
        );
        assert_eq!(
            resolve(
                None,
                None,
                Some("https://env.example"),
                Some("https://dev.example")
            ),
            "https://env.example"
        );
        // The deprecated alias still serves when it is the only one set.
        assert_eq!(
            resolve(None, None, None, Some("https://dev.example")),
            "https://dev.example"
        );
        assert_eq!(resolve(None, None, None, None), PRODUCTION);
    }

    #[test]
    fn a_blank_value_means_unset() {
        assert_eq!(resolve(Some(" "), None, Some(""), Some("  ")), PRODUCTION);
    }

    #[test]
    fn the_file_adds_and_overrides_profiles() {
        let profiles = parse_profiles(
            "[env.preprod]\napi = \"https://pre.example/\"\n[env.local]\napi = \"http://localhost:9000\"\n",
        )
        .unwrap();

        assert_eq!(
            profile_url("preprod", &profiles).unwrap(),
            "https://pre.example"
        );
        assert_eq!(
            profile_url("local", &profiles).unwrap(),
            "http://localhost:9000"
        );
        assert_eq!(profile_url("prod", &profiles).unwrap(), PRODUCTION);
    }

    #[test]
    fn an_unknown_profile_is_a_usage_error_that_lists_the_known_ones() {
        let profiles = parse_profiles("").unwrap();
        let failure = profile_url("stagging", &profiles).unwrap_err();

        assert_eq!(crate::exit::code(&failure), 2);
        assert!(
            failure.to_string().contains("prod, staging, local"),
            "{failure}"
        );
    }

    #[test]
    fn a_profile_without_an_api_says_what_it_needs() {
        let failure = parse_profiles("[env.x]\nurl = \"https://x\"\n").unwrap_err();
        assert!(failure.to_string().contains("api ="), "{failure}");
    }
}
