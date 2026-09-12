//! `portaki connectors` — what would leave this module, and whether it can.

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{Context, Result};
use clap::Parser;
use serde_json::Value;

use crate::manifest::{loader, ManifestSource};
use crate::ui;

#[derive(Debug, Parser)]
/// Arguments for `portaki connectors`.
pub struct ConnectorsArgs {}

/// Runs `portaki connectors`.
///
/// A connector is three declarations that must agree, in two files and one platform: the Rust
/// attribute, the `connectors:<id>` permission, and a credential provider bound for the
/// workspace. Nothing showed the three side by side, so the first two were checked by reading
/// and the third discovered at the first egress — as a `connector_credential_missing`.
pub fn run(_args: ConnectorsArgs) -> Result<()> {
    ui::header(
        "portaki connectors",
        "Every declared egress, with what it needs to actually call.",
    );

    let module_root = std::env::current_dir().context("current_dir")?;
    let (manifest, source) = loader::load(&module_root, None)?;
    if matches!(source, ManifestSource::Emissions) {
        ui::detail("read from the SDK emissions — no build output yet");
    }
    let granted = granted_connector_permissions(&module_root);

    if manifest.connectors.custom.is_empty() && manifest.connectors.builtin.is_empty() {
        ui::detail("this module declares no connector — it makes no outbound call");
        ui::blank();
        return Ok(());
    }

    for id in &manifest.connectors.builtin {
        ui::wrote("builtin", id);
        permission_line(id, &granted);
    }

    for connector in &manifest.connectors.custom {
        let id = string_at(connector, "id").unwrap_or_default();
        let base_url = string_at(connector, "baseUrl").unwrap_or_default();
        ui::wrote(&id, &base_url);

        for operation in connector
            .get("operations")
            .and_then(Value::as_array)
            .unwrap_or(&Vec::new())
        {
            let name = string_at(operation, "id").unwrap_or_default();
            let method = string_at(operation, "method").unwrap_or_else(|| "GET".to_string());
            let path = string_at(operation, "path").unwrap_or_default();
            ui::detail(format!("{name}  {method} {path}"));
        }

        permission_line(&id, &granted);

        // Undeclared `auth` is not `bearer`: the gateway derives the style from the provider
        // — `open-weather` goes out as a query key, for one. Naming a default here would put a
        // wrong answer next to a right one.
        let auth = string_at(connector, "auth")
            .map(|declared| format!("auth {declared}"))
            .unwrap_or_else(|| "auth derived from the provider".to_string());
        match string_at(connector, "credentialProviderId") {
            Some(provider) => ui::detail(format!("credential  {provider} · {auth}")),
            None => ui::detail(format!("credential  none declared · {auth}")),
        }
        // Only the platform knows whether that provider exists and is bound for a workspace;
        // saying so is more useful than a table of provider names that would age here.
        ui::advice("a call also needs that provider bound for the workspace — the dashboard, under Integrations");
    }

    ui::blank();
    let missing = missing_permissions(&manifest.connectors, &granted);
    if !missing.is_empty() {
        anyhow::bail!(
            "portaki.module.json grants no permission for {} — add \"connectors:{}\" to \
             permissions, or the credential resolver refuses the call",
            missing.join(", "),
            missing.first().cloned().unwrap_or_default()
        );
    }
    ui::success("every connector is declared, permitted, and named");
    ui::blank();
    Ok(())
}

fn permission_line(id: &str, granted: &BTreeSet<String>) {
    if granted.contains(id) {
        ui::detail(format!("permission  connectors:{id}"));
    } else {
        ui::detail(format!("permission  missing — add connectors:{id}"));
    }
}

/// The connector ids `portaki.module.json` grants, whatever else its permissions hold.
pub fn granted_connector_permissions(module_root: &Path) -> BTreeSet<String> {
    let Ok(raw) = std::fs::read_to_string(module_root.join("portaki.module.json")) else {
        return BTreeSet::new();
    };
    let Ok(manifest) = serde_json::from_str::<Value>(&raw) else {
        return BTreeSet::new();
    };
    manifest
        .get("permissions")
        .and_then(Value::as_array)
        .map(|permissions| {
            permissions
                .iter()
                .filter_map(Value::as_str)
                .filter_map(|permission| permission.strip_prefix("connectors:"))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// Declared connectors with no matching permission — the call the runtime will refuse.
pub fn missing_permissions(
    connectors: &portaki_sdk::manifest::ManifestConnectors,
    granted: &BTreeSet<String>,
) -> Vec<String> {
    let custom = connectors
        .custom
        .iter()
        .filter_map(|connector| string_at(connector, "id"));
    connectors
        .builtin
        .iter()
        .cloned()
        .chain(custom)
        .filter(|id| !granted.contains(id))
        .collect()
}

fn string_at(value: &Value, key: &str) -> Option<String> {
    value.get(key)?.as_str().map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use portaki_sdk::manifest::ManifestConnectors;

    #[test]
    fn only_the_connector_prefix_counts_as_a_grant() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("portaki.module.json"),
            r#"{"permissions":["kv","connectors:trmnl","email"]}"#,
        )
        .unwrap();

        let granted = granted_connector_permissions(dir.path());

        assert!(granted.contains("trmnl"));
        assert_eq!(granted.len(), 1);
    }

    #[test]
    fn a_connector_without_its_permission_is_named() {
        let connectors = ManifestConnectors {
            builtin: vec!["open-weather".to_string()],
            custom: vec![serde_json::json!({ "id": "trmnl", "baseUrl": "https://usetrmnl.com" })],
        };
        let granted = BTreeSet::from(["open-weather".to_string()]);

        assert_eq!(missing_permissions(&connectors, &granted), vec!["trmnl"]);
        assert!(missing_permissions(
            &connectors,
            &BTreeSet::from(["open-weather".to_string(), "trmnl".to_string()])
        )
        .is_empty());
    }

    /// No manifest, or no permissions at all: every connector is then unpermitted.
    #[test]
    fn a_module_without_a_catalogue_grants_nothing() {
        let dir = tempfile::tempdir().unwrap();

        assert!(granted_connector_permissions(dir.path()).is_empty());
    }
}
