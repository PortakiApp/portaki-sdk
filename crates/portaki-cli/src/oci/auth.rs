//! Registry authentication for OCI push (Scaleway + Docker config).

use std::path::PathBuf;

use anyhow::{Context, Result};
use oci_distribution::secrets::RegistryAuth;
use serde::Deserialize;

/// Resolves credentials for pushing to `registry` (host or host/path prefix).
pub fn resolve_registry_auth(registry: &str) -> Result<RegistryAuth> {
    if let Some(auth) = auth_from_env()? {
        return Ok(auth);
    }

    if let Some(auth) = auth_from_docker_config(registry)? {
        return Ok(auth);
    }

    anyhow::bail!(
        "no registry credentials: set SCW_SECRET_KEY (Scaleway secret key, username nologin) \
         or log in with docker login to the registry"
    );
}

fn auth_from_env() -> Result<Option<RegistryAuth>> {
    for key in [
        "SCW_SECRET_KEY",
        "SCALEWAY_API_KEY",
        "SCALEWAY_REGISTRY_TOKEN",
    ] {
        if let Ok(password) = std::env::var(key) {
            if !password.is_empty() {
                return Ok(Some(RegistryAuth::Basic("nologin".to_string(), password)));
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

    let host = registry_host(registry);
    let entry = auths
        .get(&host)
        .or_else(|| auths.get(registry))
        .or_else(|| {
            auths
                .iter()
                .find(|(key, _)| registry.starts_with(key.as_str()) || key.starts_with(&host))
                .map(|(_, value)| value)
        });

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

fn registry_host(registry: &str) -> String {
    registry.split('/').next().unwrap_or(registry).to_string()
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
            registry_host("rg.fr-par.scw.cloud/portaki-modules"),
            "rg.fr-par.scw.cloud"
        );
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
