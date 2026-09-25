//! `portaki permissions add` — declare a permission the way the SDK reads it.
//!
//! A permission is not written in a manifest any more: each is a feature of `portaki-sdk`, whose
//! API does not exist without it, and `portaki build` derives `permissions` from the features the
//! module turns on. Adding one is therefore a line of `Cargo.toml` — this command writes it.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use toml_edit::{Array, DocumentMut, InlineTable, Item, Value};

use crate::manifest::catalog::FEATURE_PERMISSIONS;
use crate::{ui, workspace};

#[derive(Debug, Parser)]
/// Arguments for `portaki permissions`.
pub struct PermissionsArgs {
    #[command(subcommand)]
    pub command: PermissionsCommand,
}

#[derive(Debug, Subcommand)]
pub enum PermissionsCommand {
    /// Declare a permission: turns on the portaki-sdk feature that grants it.
    Add(AddArgs),
}

#[derive(Debug, Parser)]
/// Arguments for `portaki permissions add`.
pub struct AddArgs {
    /// The permission, e.g. `email`, `kv`, `stay:guest_contact:read`.
    pub permission: String,
    /// In a repository holding several modules, the one to change.
    #[arg(long)]
    pub module: Option<String>,
}

/// Runs `portaki permissions`.
pub fn run(args: PermissionsArgs) -> Result<()> {
    let PermissionsCommand::Add(args) = args.command;
    ui::header(
        "portaki permissions add",
        "Turn on the portaki-sdk feature that declares the permission.",
    );
    let feature = feature_for(&args.permission)?;
    let member = workspace::resolve(args.module.as_deref(), None)?
        .into_iter()
        .next()
        .context("no module here")?;
    let path = member.root.join("Cargo.toml");
    let toml =
        std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    match add_feature(&toml, feature)? {
        None => ui::skipped(format!(
            "{} already declares {}",
            member.id, args.permission
        )),
        Some(updated) => {
            std::fs::write(&path, updated).with_context(|| format!("write {}", path.display()))?;
            ui::success(format!(
                "{} declares {} (portaki-sdk feature `{feature}`)",
                member.id, args.permission
            ));
            ui::next(&[(
                "portaki build",
                "writes the permission into the manifest the platform reads",
            )]);
        }
    }
    ui::blank();
    Ok(())
}

/// The feature that grants `permission`, or why none does.
fn feature_for(permission: &str) -> Result<&'static str> {
    if let Some(connector) = permission.strip_prefix(portaki_sdk::permission::CONNECTORS_PREFIX) {
        anyhow::bail!(
            "{permission} comes from declaring the connector in code — \
             #[portaki_sdk::connector] (or custom_connector) for `{connector}`; the build adds it"
        );
    }
    FEATURE_PERMISSIONS
        .iter()
        .find(|(_, granted)| *granted == permission)
        .map(|(feature, _)| *feature)
        .with_context(|| {
            format!(
                "unknown permission {permission} — known: {}, connectors:<id>",
                portaki_sdk::permission::FIXED.join(", ")
            )
        })
}

/// `Cargo.toml` with `feature` on its `portaki-sdk` dependency, `None` when it is already there.
fn add_feature(toml: &str, feature: &str) -> Result<Option<String>> {
    let mut doc: DocumentMut = toml.parse().context("parse Cargo.toml")?;
    let entry = doc
        .get_mut("dependencies")
        .and_then(|deps| deps.get_mut("portaki-sdk"))
        .context("this crate does not depend on portaki-sdk")?;
    // `portaki-sdk = "8"` has nowhere to put features: it becomes `{ version = "8" }` first.
    if let Some(version) = entry.as_str().map(str::to_string) {
        let mut table = InlineTable::new();
        table.insert("version", version.into());
        *entry = Item::Value(Value::InlineTable(table));
    }
    let table = entry
        .as_table_like_mut()
        .context("portaki-sdk is neither a version nor a table")?;
    if table.get("features").is_none() {
        table.insert("features", Item::Value(Value::Array(Array::new())));
    }
    let features = table
        .get_mut("features")
        .and_then(Item::as_array_mut)
        .context("portaki-sdk features is not an array")?;
    if features.iter().any(|f| f.as_str() == Some(feature)) {
        return Ok(None);
    }
    features.push(feature);
    Ok(Some(doc.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_permission_names_its_feature() {
        assert_eq!(feature_for("email").unwrap(), "email");
        assert_eq!(
            feature_for("stay:guest_contact:read").unwrap(),
            "stay-guest-contact"
        );
        assert!(feature_for("connectors:nuki")
            .unwrap_err()
            .to_string()
            .contains("connector"));
        assert!(feature_for("stay:read").is_err());
    }

    #[test]
    fn the_feature_joins_the_dependency_whatever_its_form() {
        let inherited = "[dependencies]\nportaki-sdk = { workspace = true, features = [\"kv\"] }\n";
        assert_eq!(
            add_feature(inherited, "email").unwrap().unwrap(),
            "[dependencies]\nportaki-sdk = { workspace = true, features = [\"kv\", \"email\"] }\n"
        );

        let bare = "[dependencies]\nportaki-sdk = \"8.4\"\n";
        assert_eq!(
            add_feature(bare, "kv").unwrap().unwrap(),
            "[dependencies]\nportaki-sdk = { version = \"8.4\", features = [\"kv\"] }\n"
        );

        let table = "[dependencies.portaki-sdk]\nversion = \"8.4\"\n";
        assert!(add_feature(table, "kv")
            .unwrap()
            .unwrap()
            .contains("features = [\"kv\"]"));
    }

    #[test]
    fn a_feature_already_on_changes_nothing() {
        let toml = "[dependencies]\nportaki-sdk = { workspace = true, features = [\"email\"] }\n";
        assert!(add_feature(toml, "email").unwrap().is_none());
    }
}
