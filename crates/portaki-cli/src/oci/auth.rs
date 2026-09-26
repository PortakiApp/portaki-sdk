//! Registry authentication for OCI push (GHCR + Docker config).

use std::path::PathBuf;

use anyhow::{Context, Result};
use oci_distribution::secrets::RegistryAuth;
use serde::Deserialize;

/// Resolves credentials for pushing to `registry` (host or host/path prefix).
pub fn resolve_registry_auth(registry: &str) -> Result<RegistryAuth> {
    if let Some(auth) = auth_from_env(registry)? {
        return Ok(auth);
    }

    if let Some(auth) = auth_from_docker_config(registry)? {
        return Ok(auth);
    }

    anyhow::bail!(
        "no registry credentials: set GITHUB_TOKEN / GHCR_TOKEN or log in with docker login ghcr.io"
    );
}

/// Comme [`resolve_registry_auth`], mais retombe sur l'anonyme au lieu d'échouer.
///
/// Réservé aux chemins en **lecture seule**. Un dépôt public se lit sans identifiants, et
/// exiger un jeton d'écriture pour résoudre un digest déjà publié interdirait la reprise d'un
/// catalogue à qui n'a pas le droit d'y pousser.
pub fn resolve_read_auth(registry: &str) -> RegistryAuth {
    resolve_registry_auth(registry).unwrap_or(RegistryAuth::Anonymous)
}

/// `GITHUB_TOKEN` / `GHCR_TOKEN` are GitHub's: they go to `ghcr.io` and nowhere else — not to
/// a registry named on the command line or in an artifact reference.
fn auth_from_env(registry: &str) -> Result<Option<RegistryAuth>> {
    if registry_host(registry) != GHCR {
        return Ok(None);
    }
    if let Ok(username) = std::env::var("OCI_USERNAME") {
        if !username.is_empty() {
            if let Ok(token) =
                std::env::var("GITHUB_TOKEN").or_else(|_| std::env::var("GHCR_TOKEN"))
            {
                if !token.is_empty() {
                    return Ok(Some(RegistryAuth::Basic(username, token)));
                }
            }
        }
    }
    for key in ["GITHUB_TOKEN", "GHCR_TOKEN"] {
        if let Ok(token) = std::env::var(key) {
            if !token.is_empty() {
                let username = std::env::var("GITHUB_ACTOR")
                    .or_else(|_| std::env::var("OCI_USERNAME"))
                    .unwrap_or_else(|_| "github".to_string());
                return Ok(Some(RegistryAuth::Basic(username, token)));
            }
        }
    }
    Ok(None)
}

#[derive(Debug, Deserialize)]
struct DockerConfig {
    auths: Option<std::collections::HashMap<String, DockerAuthEntry>>,
    #[allow(dead_code)]
    creds_store: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DockerAuthEntry {
    auth: Option<String>,
    username: Option<String>,
    password: Option<String>,
}

fn auth_from_docker_config(registry: &str) -> Result<Option<RegistryAuth>> {
    let config_path = docker_config_path();
    if !config_path.exists() {
        return Ok(None);
    }

    let raw = std::fs::read_to_string(&config_path)
        .with_context(|| format!("read {}", config_path.display()))?;
    let config: DockerConfig = serde_json::from_str(&raw).context("parse docker config.json")?;
    let auths = match config.auths {
        Some(auths) => auths,
        None => return Ok(None),
    };

    // L'hôte exact : un préfixe donnait les identifiants de `ghcr.io` à `ghcr.io.evil.example`.
    let host = registry_host(registry);
    let entry = auths
        .iter()
        .find(|(key, _)| docker_key_host(key) == host)
        .map(|(_, value)| value);

    let Some(entry) = entry else {
        return Ok(None);
    };

    if let (Some(username), Some(password)) = (&entry.username, &entry.password) {
        return Ok(Some(RegistryAuth::Basic(
            username.clone(),
            password.clone(),
        )));
    }

    if let Some(encoded) = &entry.auth {
        let decoded = base64_decode(encoded)?;
        if let Some((username, password)) = decoded.split_once(':') {
            return Ok(Some(RegistryAuth::Basic(
                username.to_string(),
                password.to_string(),
            )));
        }
    }

    Ok(None)
}

fn docker_config_path() -> PathBuf {
    if let Ok(path) = std::env::var("DOCKER_CONFIG") {
        return PathBuf::from(path).join("config.json");
    }
    dirs_home().join(".docker/config.json")
}

fn dirs_home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

const GHCR: &str = "ghcr.io";

pub fn registry_host(registry: &str) -> String {
    registry.split('/').next().unwrap_or(registry).to_string()
}

/// `https://ghcr.io/v1/` or `ghcr.io` — Docker writes both — down to the host.
fn docker_key_host(key: &str) -> String {
    let bare = key
        .strip_prefix("https://")
        .or_else(|| key.strip_prefix("http://"))
        .unwrap_or(key);
    registry_host(bare)
}

fn base64_decode(input: &str) -> Result<String> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(input.trim())
        .context("decode docker auth base64")?;
    String::from_utf8(bytes).context("docker auth utf8")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_host_strips_path() {
        assert_eq!(
            registry_host("ghcr.io/portakiapp/portaki-modules"),
            "ghcr.io"
        );
    }

    #[test]
    fn github_tokens_only_go_to_ghcr() {
        std::env::set_var("GHCR_TOKEN", "secret");
        assert!(auth_from_env("ghcr.io/portakiapp").unwrap().is_some());
        assert!(auth_from_env("registry.evil.example/x").unwrap().is_none());
        assert!(auth_from_env("ghcr.io.evil.example/x").unwrap().is_none());
        std::env::remove_var("GHCR_TOKEN");
    }

    #[test]
    fn docker_config_keys_match_the_exact_host() {
        assert_eq!(docker_key_host("https://ghcr.io/v1/"), "ghcr.io");
        assert_eq!(docker_key_host("ghcr.io"), "ghcr.io");
        assert_ne!(docker_key_host("ghcr.io.evil.example"), "ghcr.io");
        assert_ne!(docker_key_host("ghcr"), "ghcr.io");
    }

    #[test]
    fn base64_decode_username_password() {
        let encoded = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            "nologin:secret-key",
        );
        let decoded = base64_decode(&encoded).unwrap();
        assert_eq!(decoded, "nologin:secret-key");
    }
}
