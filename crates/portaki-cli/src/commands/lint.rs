//! `portaki lint` — validate manifest and i18n bundles.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use serde_json::from_reader;

use crate::manifest::collect_emissions;
use crate::manifest::{find_emissions_dir, generate_manifest, validate_manifest};
use crate::ui;
use portaki_sdk::manifest::{ManifestCommand, ModuleManifest};

#[derive(Debug, Parser)]
/// Arguments for `portaki lint`.
pub struct LintArgs {
    /// Path to `manifest.json` (defaults to `target/portaki/manifest.json`).
    #[arg(long)]
    pub manifest: Option<PathBuf>,

    /// `check` enchaîne sur `lint` : un second en-tête ferait croire à deux commandes.
    #[arg(skip)]
    pub nested: bool,
}

/// Runs `portaki lint`.
pub fn run(args: LintArgs) -> Result<()> {
    if !args.nested {
        ui::header(
            "portaki lint",
            "Check that everything the manifest names actually resolves.",
        );
    }

    let module_root = std::env::current_dir().context("current_dir")?;
    let manifest_path = args
        .manifest
        .unwrap_or_else(|| module_root.join("target/portaki/manifest.json"));

    let reading = ui::step("reading the manifest");
    let manifest = if manifest_path.exists() {
        let file = std::fs::File::open(&manifest_path)?;
        let manifest = from_reader::<_, ModuleManifest>(file)?;
        reading.done(format!("read {}", manifest_path.display()));
        manifest
    } else if let Some(emissions_dir) = find_emissions_dir(&module_root) {
        let emissions = collect_emissions(&emissions_dir)?;
        let manifest = generate_manifest(
            &emissions,
            "fr-FR",
            &["fr-FR".to_string(), "en-US".to_string()],
        )?;
        reading.done("read the SDK emissions (no build output yet)");
        manifest
    } else {
        reading.abandon();
        anyhow::bail!("no manifest or emissions found — run portaki build first");
    };

    let checking = ui::step("checking capability ids, connector bindings, and i18n keys");
    validate_manifest(&manifest, &module_root.join("i18n")).map_err(|failure| {
        checking.abandon();
        failure
    })?;
    assert_known_permissions(&module_root).map_err(|failure| {
        checking.abandon();
        failure
    })?;
    // Le manifeste que `publish` enverra, écrit du code par `portaki build` : le registre le
    // refuserait hors schéma, autant le dire ici.
    portaki_test_utils::conformance::Module::at(&module_root)
        .check_manifest()
        .map_err(|findings| {
            checking.abandon();
            anyhow::anyhow!("{findings}")
        })?;
    assert_versions_agree(&module_root, &manifest).map_err(|failure| {
        checking.abandon();
        failure
    })?;
    assert_feeds_valid(&module_root).map_err(|failure| {
        checking.abandon();
        failure
    })?;
    checking.done(format!("{} passes", manifest.id));
    for name in host_like_guest_commands(&manifest.commands) {
        ui::warn(format!(
            "command {name} is open to guests but reads like a host operation — \
             drop `guest` unless a guest really calls it"
        ));
    }
    ui::detail("capability ids, connector bindings and i18n keys all resolve");
    ui::blank();
    Ok(())
}

/// A permission the manifest schema does not know.
///
/// The registry refuses it at publication, against the schema of the SDK the module targets.
/// Saying so here is cheaper than a refused publish — and a misspelt `stay:guest_contact:read`
/// would otherwise build, deploy, and quietly leave the guest's contact details empty.
fn assert_known_permissions(module_root: &std::path::Path) -> Result<()> {
    let unknown = unknown_permissions(&declared_permissions(module_root));
    if unknown.is_empty() {
        return Ok(());
    }
    anyhow::bail!(
        "portaki.module.json declares unknown permission(s) {} — known: {}, connectors:<id>",
        unknown.join(", "),
        portaki_sdk::permission::FIXED.join(", ")
    )
}

/// The `permissions` of `portaki.module.json`, empty when the file or the field is absent.
fn declared_permissions(module_root: &std::path::Path) -> Vec<String> {
    std::fs::read_to_string(module_root.join("portaki.module.json"))
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .and_then(|manifest| manifest.get("permissions").cloned())
        .and_then(|permissions| serde_json::from_value::<Vec<String>>(permissions).ok())
        .unwrap_or_default()
}

fn unknown_permissions(declared: &[String]) -> Vec<String> {
    declared
        .iter()
        .filter(|permission| !portaki_sdk::permission::is_known(permission))
        .cloned()
        .collect()
}

/// `feeds` — the modules this one supplies — must name another module and say what, in fr and en.
///
/// The catalogue turns it into a sentence (« Nuki fournit le code clavier au module Accès ») :
/// a missing locale or a typo'd id would print a hole or a dead link on the public sheet.
fn assert_feeds_valid(module_root: &std::path::Path) -> Result<()> {
    let Some(manifest) = std::fs::read_to_string(module_root.join("portaki.module.json"))
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
    else {
        return Ok(());
    };
    let problems = feeds_problems(&manifest);
    if problems.is_empty() {
        return Ok(());
    }
    anyhow::bail!("portaki.module.json feeds: {}", problems.join("; "))
}

fn feeds_problems(manifest: &serde_json::Value) -> Vec<String> {
    let Some(feeds) = manifest.get("feeds") else {
        return vec![];
    };
    let Some(feeds) = feeds.as_array() else {
        return vec!["must be an array".to_string()];
    };
    let own_id = manifest
        .get("id")
        .and_then(|id| id.as_str())
        .unwrap_or_default();
    let mut problems = Vec::new();
    for (index, feed) in feeds.iter().enumerate() {
        let module = feed
            .get("module")
            .and_then(|m| m.as_str())
            .unwrap_or_default();
        let valid_id = module.starts_with(|c: char| c.is_ascii_lowercase())
            && module
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        if !valid_id {
            problems.push(format!("[{index}].module \"{module}\" is not a module id"));
        } else if module == own_id {
            problems.push(format!("[{index}].module names the module itself"));
        }
        for locale in ["fr", "en"] {
            let text = feed
                .pointer(&format!("/what/{locale}"))
                .and_then(|t| t.as_str())
                .unwrap_or_default();
            if text.trim().is_empty() {
                problems.push(format!("[{index}].what.{locale} is missing"));
            }
        }
    }
    problems
}

/// La crate et le manifeste doivent annoncer la même version.
///
/// `release-please` incrémente les deux ; si l'un des deux passe à travers, un artefact part
/// sous un numéro que rien d'autre ne porte, et la version publiée cesse de désigner le code
/// qu'elle contient. Le contrôle vivait dans le script bash d'un dépôt — il appartient au lint.
fn assert_versions_agree(module_root: &std::path::Path, manifest: &ModuleManifest) -> Result<()> {
    let cargo = module_root.join("Cargo.toml");
    let Ok(text) = std::fs::read_to_string(&cargo) else {
        // Pas de crate ici : un manifeste peut être linté seul.
        return Ok(());
    };
    let Some(declared) = crate_version(&text) else {
        return Ok(());
    };
    if declared == manifest.version {
        return Ok(());
    }
    anyhow::bail!(
        "Cargo.toml says {declared} and the manifest says {} — a release bumps both, so one of \
         them was missed",
        manifest.version
    )
}

/// La version de `[package]`, sans analyseur TOML pour un champ.
///
/// Lue dans sa seule section : une `version` de dépendance ne doit pas passer pour celle de la
/// crate. Une version héritée de l'espace de travail n'est pas lisible ici, et se lit comme
/// absente plutôt que comme un désaccord.
fn crate_version(cargo: &str) -> Option<String> {
    let mut in_package = false;
    for line in cargo.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_package = line == "[package]";
            continue;
        }
        if !in_package {
            continue;
        }
        if let Some(rest) = line.strip_prefix("version") {
            let rest = rest.trim_start().strip_prefix('=')?.trim();
            return rest
                .strip_prefix('"')
                .and_then(|rest| rest.strip_suffix('"'))
                .map(str::to_string);
        }
    }
    None
}

/// Guest commands named like the host's own gestures — a finding, not a failure: the name is
/// only a hint, and the module may have a reason.
fn host_like_guest_commands(commands: &[ManifestCommand]) -> Vec<&str> {
    const HOST: [&str; 5] = [
        "updateConfig",
        "resolve",
        "updateStatus",
        "seedDefaults",
        "replaceItems",
    ];
    commands
        .iter()
        .filter(|command| command.guest)
        .map(|command| command.name.as_str())
        .filter(|name| HOST.contains(name) || name.starts_with("task"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_guest_command_named_like_a_host_one_is_flagged() {
        let command = |name: &str, guest| ManifestCommand {
            name: name.to_string(),
            r#fn: String::new(),
            args: None,
            params: None,
            guest,
        };
        let commands = [
            command("taskToggle", true),
            command("submit", true),
            command("updateConfig", false),
        ];
        assert_eq!(host_like_guest_commands(&commands), ["taskToggle"]);
    }

    #[test]
    fn the_crate_version_is_read_from_its_own_section() {
        let cargo = r#"
[package]
name = "weather"
version = "0.3.24"

[dependencies]
serde = { version = "1", features = ["derive"] }
"#;

        assert_eq!(crate_version(cargo).as_deref(), Some("0.3.24"));
    }

    /// Une `version` de dépendance ne doit pas passer pour celle de la crate.
    #[test]
    fn a_dependency_version_is_not_mistaken_for_the_crate() {
        let cargo = "[dependencies]\nserde = { version = \"1\" }\n";

        assert!(crate_version(cargo).is_none());
    }

    #[test]
    fn the_guest_contact_permission_is_accepted() {
        let declared = vec![
            "kv".to_string(),
            "stay:guest_contact:read".to_string(),
            "connectors:nuki".to_string(),
        ];

        assert!(unknown_permissions(&declared).is_empty());
    }

    /// `stay:read` est un scope de jeton, pas une permission de manifeste : le séjour se lit sans.
    #[test]
    fn an_unknown_or_misspelt_permission_is_named() {
        let declared = vec![
            "stay:read".to_string(),
            "stay:guest-contact:read".to_string(),
            "email".to_string(),
        ];

        assert_eq!(
            unknown_permissions(&declared),
            vec![
                "stay:read".to_string(),
                "stay:guest-contact:read".to_string()
            ]
        );
    }

    #[test]
    fn permissions_are_read_from_the_catalogue_manifest() {
        let root = std::env::temp_dir().join(format!("portaki-lint-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("portaki.module.json"),
            r#"{"id":"checkin","permissions":["stay:guest_contact:read","stay:read"]}"#,
        )
        .unwrap();

        let failure = assert_known_permissions(&root).unwrap_err().to_string();
        std::fs::remove_dir_all(&root).ok();

        assert!(failure.contains("stay:read"), "{failure}");
        assert!(!failure.contains("unknown permission(s) stay:guest_contact:read"));
    }

    #[test]
    fn a_well_formed_feed_passes() {
        let manifest = serde_json::json!({
            "id": "nuki",
            "feeds": [{ "module": "access-guide", "what": { "fr": "le code clavier", "en": "the keypad code" } }]
        });

        assert!(feeds_problems(&manifest).is_empty());
        assert!(feeds_problems(&serde_json::json!({ "id": "nuki" })).is_empty());
    }

    #[test]
    fn a_bad_feed_is_named() {
        let manifest = serde_json::json!({
            "id": "nuki",
            "feeds": [
                { "module": "Access_Guide", "what": { "fr": "le code", "en": "the code" } },
                { "module": "nuki", "what": { "fr": "x", "en": "x" } },
                { "module": "access-guide", "what": { "fr": "le code clavier" } }
            ]
        });

        assert_eq!(
            feeds_problems(&manifest),
            vec![
                "[0].module \"Access_Guide\" is not a module id".to_string(),
                "[1].module names the module itself".to_string(),
                "[2].what.en is missing".to_string(),
            ]
        );
    }

    #[test]
    fn a_workspace_inherited_version_is_left_alone() {
        let cargo = "[package]\nname = \"weather\"\nversion.workspace = true\n";

        assert!(crate_version(cargo).is_none());
    }
}
