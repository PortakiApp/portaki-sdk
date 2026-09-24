//! Ce qu'un module dit de lui-même, avec ou sans `portaki.module.json`.
//!
//! Le fichier n'est plus nécessaire : le code déclare le module (`portaki_module!`, `#[surface]`,
//! `#[email]`, les features de `portaki-sdk`) et `portaki build` en écrit le catalogue. Tant
//! qu'un module le garde, il reste lu et l'emporte ; sans lui, `Cargo.toml` dit l'id et la
//! version, et le build le reste.

use std::path::Path;

use anyhow::{Context, Result};

/// Le manifeste écrit à la main, facultatif.
pub const MODULE_MANIFEST: &str = "portaki.module.json";

/// Un dossier est un module s'il garde un manifeste, ou si son crate dépend de `portaki-sdk`.
pub fn is_module(root: &Path) -> bool {
    root.join(MODULE_MANIFEST).is_file()
        || cargo_package(root).is_some_and(|doc| {
            doc.get("dependencies")
                .and_then(|deps| deps.get("portaki-sdk"))
                .is_some()
        })
}

/// L'id et la version du module : ceux du manifeste s'il les porte, ceux du crate sinon.
///
/// Le crate et le module portent le même nom et la même version dans tout le catalogue ;
/// release-please les monte ensemble.
pub fn coordinates(root: &Path) -> Option<(String, String)> {
    let from_manifest = std::fs::read_to_string(root.join(MODULE_MANIFEST))
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .and_then(|value| {
            Some((
                value.get("id")?.as_str()?.to_string(),
                value.get("version")?.as_str()?.to_string(),
            ))
        });
    from_manifest.or_else(|| {
        let doc = cargo_package(root)?;
        let package = doc.get("package")?;
        Some((
            package.get("name")?.as_str()?.to_string(),
            package.get("version")?.as_str()?.to_string(),
        ))
    })
}

/// L'id seul, pour désigner le module.
pub fn module_id(root: &Path) -> Option<String> {
    coordinates(root).map(|(id, _)| id)
}

/// Le manifeste de départ : le fichier écrit à la main, ou `{ id, version }` tirés du crate.
///
/// L'appelant le complète ensuite de ce que le build a émis (catalogue, déclarations).
pub fn source_manifest(root: &Path) -> Result<String> {
    let path = root.join(MODULE_MANIFEST);
    if path.is_file() {
        return std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()));
    }
    let (id, version) = coordinates(root).with_context(|| {
        format!(
            "{} is not a module — no {MODULE_MANIFEST} and no [package] name/version in Cargo.toml",
            root.display()
        )
    })?;
    Ok(serde_json::to_string_pretty(
        &serde_json::json!({ "id": id, "version": version }),
    )?)
}

fn cargo_package(root: &Path) -> Option<toml_edit::DocumentMut> {
    let raw = std::fs::read_to_string(root.join("Cargo.toml")).ok()?;
    let doc: toml_edit::DocumentMut = raw.parse().ok()?;
    doc.get("package")?;
    Some(doc)
}

#[cfg(test)]
mod tests {
    use super::{coordinates, is_module, source_manifest, MODULE_MANIFEST};

    const CRATE: &str = "[package]\nname = \"issue-report\"\nversion = \"0.6.0\"\n\n[dependencies]\nportaki-sdk = { workspace = true }\n";

    #[test]
    fn a_crate_on_the_sdk_is_a_module_without_any_json() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("Cargo.toml"), CRATE).expect("cargo");

        assert!(is_module(dir.path()));
        assert_eq!(
            coordinates(dir.path()),
            Some(("issue-report".into(), "0.6.0".into()))
        );
        let manifest: serde_json::Value =
            serde_json::from_str(&source_manifest(dir.path()).expect("source")).expect("json");
        assert_eq!(
            manifest,
            serde_json::json!({ "id": "issue-report", "version": "0.6.0" })
        );
    }

    #[test]
    fn a_kept_manifest_still_wins() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("Cargo.toml"), CRATE).expect("cargo");
        let kept = r#"{"id":"renamed","version":"1.0.0","icon":"x"}"#;
        std::fs::write(dir.path().join(MODULE_MANIFEST), kept).expect("json");

        assert_eq!(
            coordinates(dir.path()),
            Some(("renamed".into(), "1.0.0".into()))
        );
        assert_eq!(source_manifest(dir.path()).expect("source"), kept);
    }

    #[test]
    fn a_workspace_root_or_a_plain_crate_is_not_a_module() {
        let workspace = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            workspace.path().join("Cargo.toml"),
            "[workspace]\nmembers = []\n[workspace.dependencies]\nportaki-sdk = \"6\"\n",
        )
        .expect("cargo");
        assert!(!is_module(workspace.path()));

        let plain = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            plain.path().join("Cargo.toml"),
            "[package]\nname = \"x\"\nversion = \"0.1.0\"\n",
        )
        .expect("cargo");
        assert!(!is_module(plain.path()));
        assert!(source_manifest(tempfile::tempdir().expect("t").path()).is_err());
    }
}
