//! `portaki connectors` — what would leave this module, and whether it can.

use anyhow::{Context, Result};
use clap::Parser;
use serde_json::{json, Value};

use crate::manifest::{loader, ManifestSource};
use crate::ui;

#[derive(Debug, Parser)]
/// Arguments for `portaki connectors`.
pub struct ConnectorsArgs {
    #[command(flatten)]
    pub modules: crate::workspace::ModuleArgs,
}

/// Runs `portaki connectors`.
///
/// A connector needs its Rust attribute and a credential provider bound for the workspace. The
/// `connectors:<id>` permission follows from the attribute — `portaki build` writes it — so only
/// the provider is left to discover, and it was discovered at the first egress, as a
/// `connector_credential_missing`.
pub fn run(args: ConnectorsArgs) -> Result<()> {
    ui::header(
        "portaki connectors",
        "Every declared egress, with what it needs to actually call.",
    );
    let mut modules = Vec::new();
    args.modules.for_each(|member| {
        let connectors = show(&std::env::current_dir().context("current_dir")?)?;
        modules.push(json!({ "id": member.id, "connectors": connectors }));
        Ok(())
    })?;
    if ui::json() {
        ui::emit(&json!({ "schemaVersion": 1, "modules": modules }));
    }
    Ok(())
}

/// Shows the connectors of the module in `module_root`, and returns them as `--json` renders
/// them.
fn show(module_root: &std::path::Path) -> Result<Vec<Value>> {
    let (manifest, source) = loader::load(module_root, None)?;
    if matches!(source, ManifestSource::Emissions) {
        ui::detail("read from the SDK emissions — no build output yet");
    }

    let mut found = Vec::new();
    if manifest.connectors.custom.is_empty() && manifest.connectors.builtin.is_empty() {
        ui::detail("this module declares no connector — it makes no outbound call");
        ui::blank();
        return Ok(found);
    }

    for id in &manifest.connectors.builtin {
        ui::wrote("builtin", id);
        permission_line(id);
        found.push(json!({
            "id": id,
            "kind": "builtin",
            "permission": format!("connectors:{id}"),
        }));
    }

    for connector in &manifest.connectors.custom {
        let id = string_at(connector, "id").unwrap_or_default();
        let base_url = string_at(connector, "baseUrl").unwrap_or_default();
        ui::wrote(&id, &base_url);

        let mut operations = Vec::new();
        for operation in connector
            .get("operations")
            .and_then(Value::as_array)
            .unwrap_or(&Vec::new())
        {
            let name = string_at(operation, "id").unwrap_or_default();
            let method = string_at(operation, "method").unwrap_or_else(|| "GET".to_string());
            let path = string_at(operation, "path").unwrap_or_default();
            ui::detail(format!("{name}  {method} {path}"));
            operations.push(json!({ "id": name, "method": method, "path": path }));
        }

        permission_line(&id);

        // Undeclared `auth` is not `bearer`: the gateway derives the style from the provider
        // — `open-weather` goes out as a query key, for one. Naming a default here would put a
        // wrong answer next to a right one.
        let auth = string_at(connector, "auth");
        let provider = string_at(connector, "credentialProviderId");
        let auth_text = auth
            .as_ref()
            .map(|declared| format!("auth {declared}"))
            .unwrap_or_else(|| "auth derived from the provider".to_string());
        match &provider {
            Some(provider) => ui::detail(format!("credential  {provider} · {auth_text}")),
            None => ui::detail(format!("credential  none declared · {auth_text}")),
        }
        // Only the platform knows whether that provider exists and is bound for a workspace;
        // saying so is more useful than a table of provider names that would age here.
        ui::advice("a call also needs that provider bound for the workspace — the dashboard, under Integrations");
        found.push(json!({
            "id": id,
            "kind": "custom",
            "baseUrl": base_url,
            "operations": operations,
            "permission": format!("connectors:{id}"),
            "credentialProviderId": provider,
            "auth": auth,
        }));
    }

    ui::blank();
    ui::success("every connector is declared, permitted, and named");
    ui::blank();
    Ok(found)
}

/// The permission the declaration grants — `portaki build` writes it from the attribute.
fn permission_line(id: &str) {
    ui::detail(format!("permission  connectors:{id}"));
}

fn string_at(value: &Value, key: &str) -> Option<String> {
    value.get(key)?.as_str().map(str::to_string)
}
