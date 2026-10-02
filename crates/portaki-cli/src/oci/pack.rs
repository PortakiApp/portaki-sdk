//! Collects module files into OCI layers for push.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use oci_distribution::client::ImageLayer;
use serde::Deserialize;

pub const MANIFEST_MEDIA: &str = "application/vnd.portaki.manifest+json";
pub const SDK_MANIFEST_MEDIA: &str = "application/vnd.portaki.sdk.manifest+json";
const WASM_MEDIA: &str = "application/wasm";
const I18N_MEDIA: &str = "application/vnd.portaki.i18n+json";
const MIGRATIONS_BUNDLE_MEDIA: &str = "application/vnd.portaki.migrations+json";
pub const MIGRATIONS_BUNDLE: &str = "migrations.bundle.json";
const OPERATIONS_BUNDLE_MEDIA: &str = "application/vnd.portaki.operations+json";
pub const OPERATIONS_BUNDLE: &str = "operations.bundle.json";
/// Guest surfaces pre-rendered on sample config, committed by the module and served by the
/// registry on the public catalogue sheet — never rendered on a host's data.
const PREVIEWS_MEDIA: &str = "application/vnd.portaki.previews+json";
pub const PREVIEWS: &str = "previews.json";

/// OCI host-catalog layer (`portaki.module.json` freeze) — consumed by API / install.
pub const PUBLISH_MANIFEST: &str = "publish-manifest.json";
/// SDK emissions manifest (`target/portaki/manifest.json`) — wasm surfaces, capabilities, i18n keys.
pub const SDK_MANIFEST: &str = "manifest.json";

/// One blob to upload with its OCI media type.
#[derive(Debug, Clone)]
pub struct PushLayer {
    pub path: PathBuf,
    pub media_type: String,
}

/// Module coordinates read from the publish manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleCoordinates {
    pub id: String,
    pub version: String,
}

/// Parsed publish / SDK manifest (`id` + `version` for OCI tag).
#[derive(Debug, Deserialize)]
struct ArtifactManifest {
    id: String,
    version: String,
}

/// Path to the frozen manifest produced by `portaki build`.
pub fn publish_manifest_path(artifact_dir: &Path) -> PathBuf {
    artifact_dir.join(PUBLISH_MANIFEST)
}

/// Assembles `target/portaki/publish-manifest.json`: `portaki.module.json` when the module keeps
/// one, `{ id, version }` from its crate otherwise, completed by what the build emitted.
pub fn assemble_publish_manifest(module_root: &Path, artifact_dir: &Path) -> Result<PathBuf> {
    fs::create_dir_all(artifact_dir).context("create artifact dir")?;
    let dest = publish_manifest_path(artifact_dir);
    let sdk_path = artifact_dir.join("manifest.json");

    let raw = crate::manifest::source::source_manifest(module_root)?;
    let raw = match fs::read_to_string(artifact_dir.join("catalog.json")) {
        Ok(catalog) => crate::manifest::catalog::fill_catalog(&raw, &catalog)?,
        Err(_) => raw,
    };
    let stamped = stamp_sdk_version(&raw, resolved_sdk_version(module_root)?)?;

    // What the build emitted travels all the way to the published manifest now, and no longer
    // only as far as the sandbox. The platform reads that manifest: without the operations it
    // cannot know which module to ask what, and has to query them all blindly only to collect
    // a `wasm_handler_not_found` from the ones that have nothing to say.
    //
    let stamped = if !sdk_path.exists() {
        stamped
    } else {
        let built = fs::read_to_string(&sdk_path)
            .with_context(|| format!("read {}", sdk_path.display()))?;
        stamp_built_declarations(&stamped, &built)?
    };

    fs::write(&dest, stamped).with_context(|| format!("write {}", dest.display()))?;
    Ok(dest)
}

/// The version of `portaki-sdk` <strong>actually linked</strong>, read from the graph cargo
/// resolved.
///
/// Not the declared one: modules depend on the SDK through `workspace = true`, whose constraint
/// is `*`. What matters when choosing a bundle of contracts is what the binary was compiled
/// against, not what someone wrote next to it.
///
/// Returns `None` when cargo does not answer or the SDK is not in the graph — a module that does
/// not depend on it does not get a version invented for it.
pub(crate) fn resolved_sdk_version(module_root: &Path) -> Result<Option<String>> {
    let output = std::process::Command::new("cargo")
        .args(["metadata", "--format-version", "1"])
        .current_dir(module_root)
        .output();
    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => return Ok(None),
    };
    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).context("parse cargo metadata")?;
    let found = metadata
        .get("packages")
        .and_then(|p| p.as_array())
        .into_iter()
        .flatten()
        .find(|p| p.get("name").and_then(|n| n.as_str()) == Some("portaki-sdk"))
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .map(str::to_string);
    Ok(found)
}

/// The features the module turns on for `portaki-sdk`, read from its own resolved `Cargo.toml`.
///
/// The ones it declares and not the ones cargo unifies across a workspace: a module does not
/// claim `email` because its neighbour sends some.
pub(crate) fn sdk_features(module_root: &Path) -> Result<Vec<String>> {
    let output = std::process::Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .current_dir(module_root)
        .output();
    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => return Ok(Vec::new()),
    };
    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).context("parse cargo metadata")?;
    let own = module_root
        .join("Cargo.toml")
        .canonicalize()
        .unwrap_or_else(|_| module_root.join("Cargo.toml"));
    let features = metadata["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|p| p["manifest_path"].as_str().map(PathBuf::from) == Some(own.clone()))
        .and_then(|p| p["dependencies"].as_array())
        .into_iter()
        .flatten()
        .filter(|d| d["name"] == "portaki-sdk" && d["kind"].is_null())
        .flat_map(|d| d["features"].as_array().cloned().unwrap_or_default())
        .filter_map(|f| f.as_str().map(str::to_string))
        .collect();
    Ok(features)
}

/// Copies what the build emitted into the manifest sent to the sandbox: the surfaces, the
/// queries, the commands and the entities.
///
/// Two manifests coexist and do not say the same thing. `portaki.module.json` describes the
/// dashboard's navigation: its `hostSurfaces` carry a `pathSegment`, which is a piece of URL.
/// The manifest the build emits describes what the binary really exports:
/// `surfaces.host[].id` is `main`, and the symbol that goes with it is `render_host_main`.
///
/// The sandbox only received the first one. It inferred from it a surface id equal to the
/// `pathSegment` — `access-guide` —, the runtime looked for `render_host_access_guide`, and no
/// module exports that: the twenty modules that declare a host surface failed on
/// `wasm_handler_not_found`. Production, for its part, works because the dashboard sends `main`.
///
/// Fixing the twenty hand-written manifests would be a second source of truth for something the
/// build already knows. So we carry over what it emitted.
///
/// The operations follow the same path, for the same reason: `#[portaki_sdk::query]` and
/// `#[portaki_sdk::command]` only exist in the emitted manifest, and a wasm module exports only
/// `portaki_query` / `portaki_command` — the binary cannot say what it serves. Without them, the
/// sandbox could offer no list of operations at all and fell back on a free-text field, where a
/// typo was only discovered at `handler_not_found`.
pub fn stamp_built_declarations(raw: &str, built_manifest: &str) -> Result<String> {
    let built: serde_json::Value =
        serde_json::from_str(built_manifest).context("parse built manifest")?;
    let carried: Vec<(&str, &serde_json::Value)> = BUILT_DECLARATIONS
        .iter()
        .filter_map(|key| built.get(*key).map(|value| (*key, value)))
        .collect();
    let emails = built
        .get("emails")
        .and_then(serde_json::Value::as_array)
        .filter(|emails| !emails.is_empty());
    if carried.is_empty() && emails.is_none() {
        return Ok(raw.to_string());
    }
    let mut manifest: serde_json::Value =
        serde_json::from_str(raw).context("parse module manifest")?;
    if let Some(object) = manifest.as_object_mut() {
        for (key, value) in carried {
            object.insert(key.to_string(), value.clone());
        }
        if let Some(emails) = emails {
            merge_emails(object, emails);
        }
    }
    serde_json::to_string_pretty(&manifest).context("serialise module manifest")
}

/// Pours the build's `#[email]`s into `emails[]`, by `id`, and grants the `email` permission.
///
/// A merge and not a replacement: an email the code does not describe yet — `ical-sync` emits
/// its own during a query, not a command — stays as the manifest writes it. For an `id` present
/// on both sides, the build wins field by field; what it does not say, a `description` for
/// instance, is kept.
///
/// The permission follows: a module that declares a send must be able to perform one, and the
/// permission is what compliance reads to require that an `email.send` has been observed.
fn merge_emails(
    manifest: &mut serde_json::Map<String, serde_json::Value>,
    built: &[serde_json::Value],
) {
    let declared = manifest
        .entry("emails")
        .or_insert_with(|| serde_json::Value::Array(Vec::new()));
    if !declared.is_array() {
        *declared = serde_json::Value::Array(Vec::new());
    }
    let declared = declared.as_array_mut().expect("emails is an array");
    for email in built {
        let Some(fields) = email.as_object() else {
            continue;
        };
        match declared
            .iter_mut()
            .find(|entry| entry.get("id").is_some() && entry.get("id") == email.get("id"))
            .and_then(serde_json::Value::as_object_mut)
        {
            Some(entry) => entry.extend(fields.clone()),
            None => declared.push(email.clone()),
        }
    }

    let permissions = manifest
        .entry("permissions")
        .or_insert_with(|| serde_json::Value::Array(Vec::new()));
    if let Some(permissions) = permissions.as_array_mut() {
        if !permissions.iter().any(|p| p == "email") {
            permissions.push("email".into());
        }
    }
}

/// What only the build can say, and which the sandbox must therefore receive from it.
///
/// `entities` too: `repo.find` checks that the entity is declared in the manifest, and the
/// sandbox runtime has only this one. Without them, every module with typed storage failed in
/// the sandbox while its published image, which carries the build's manifest, worked.
///
/// `dispatchExamples` for the same reason: `example(…)` only exists in what the build emits, and
/// the sandbox's "Exécuter" tab reads them from this manifest.
///
/// `connectors` because the orchestrator lends a key (BYOK or pool) only to a module whose
/// published manifest declares the connector's `credentialProviderId`. `#[custom_connector]`
/// only exists in the build's manifest: without it, every keyed call was refused
/// `PROVIDER_NOT_DECLARED`, while the runtime, reading the build's manifest, saw the connector.
const BUILT_DECLARATIONS: [&str; 6] = [
    "surfaces",
    "queries",
    "commands",
    "entities",
    "dispatchExamples",
    "connectors",
];

/// Writes `requiresModuleSdk` into the manifest, or refuses if the author announces another one.
///
/// The field has been in the schema for a long time and <strong>no module was filling it
/// in</strong>: the platform therefore had nothing to choose the right set of contracts with.
/// Writing it at build time makes it exact by construction rather than by discipline.
///
/// Declaring it is still preferable where one can, and the refusal no longer says "remove the
/// field". The twenty-one modules of the catalogue write it: release-please only attributes
/// commits by path, so an SDK bump that touched only a workspace manifest published none of
/// them. An author who runs into this refusal has two ways out, and both are legitimate — align
/// the manifest, or pin the other version of the crate.
pub fn stamp_sdk_version(raw: &str, resolved: Option<String>) -> Result<String> {
    let Some(resolved) = resolved else {
        return Ok(raw.to_string());
    };
    let mut manifest: serde_json::Value =
        serde_json::from_str(raw).context("parse module manifest")?;
    match manifest.get("requiresModuleSdk").and_then(|v| v.as_str()) {
        Some(declared) if declared != resolved => anyhow::bail!(
            "portaki.module.json declares requiresModuleSdk {declared} but the build linked \
             portaki-sdk {resolved} — set the manifest to {resolved}, or pin the crate to \
             {declared}"
        ),
        _ => {}
    }
    if let Some(object) = manifest.as_object_mut() {
        // `sdkVersion` is what devapi and the registry read in order to say "SDK too old";
        // `requiresModuleSdk` stays for the readers that have always read it. Same value.
        object.insert("sdkVersion".to_string(), resolved.clone().into());
        object.insert("requiresModuleSdk".to_string(), resolved.into());
    }
    serde_json::to_string_pretty(&manifest).context("serialise module manifest")
}

/// Reads module id/version from `publish-manifest.json` under `artifact_dir`.
pub fn read_module_coordinates(
    _module_root: &Path,
    artifact_dir: &Path,
) -> Result<ModuleCoordinates> {
    let manifest_path = publish_manifest_path(artifact_dir);
    let raw = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("read {}", manifest_path.display()))?;
    let manifest: ArtifactManifest =
        serde_json::from_str(&raw).context("parse publish-manifest.json")?;
    Ok(ModuleCoordinates {
        id: manifest.id,
        version: manifest.version,
    })
}

/// Reads id/version from the sources — `portaki.module.json` or the crate —, without a build.
///
/// This is what the publication job checks against the artifact another job built: the sources
/// name the module, not the code that compiled it.
pub fn read_source_coordinates(module_root: &Path) -> Result<ModuleCoordinates> {
    let (id, version) = crate::manifest::source::coordinates(module_root).with_context(|| {
        format!(
            "{} carries no module id/version — no portaki.module.json and no [package] in Cargo.toml",
            module_root.display()
        )
    })?;
    Ok(ModuleCoordinates { id, version })
}

/// Discovers wasm + publish manifest + optional SDK manifest + i18n layers.
pub fn collect_push_layers(module_root: &Path, artifact_dir: &Path) -> Result<Vec<PushLayer>> {
    let coords = read_module_coordinates(module_root, artifact_dir)?;
    let mut layers = Vec::new();

    let catalog_layer_path = publish_manifest_path(artifact_dir);
    if !catalog_layer_path.exists() {
        anyhow::bail!(
            "missing {} — run portaki build before publish",
            catalog_layer_path.display()
        );
    }
    layers.push(PushLayer {
        path: catalog_layer_path.clone(),
        media_type: MANIFEST_MEDIA.to_string(),
    });

    let sdk_layer_path = artifact_dir.join(SDK_MANIFEST);
    if sdk_layer_path.exists() && publish_layer_is_host_catalog_shape(&catalog_layer_path)? {
        layers.push(PushLayer {
            path: sdk_layer_path,
            media_type: SDK_MANIFEST_MEDIA.to_string(),
        });
    }

    let wasm_path = find_wasm_artifact(module_root, &coords.id)?;
    layers.push(PushLayer {
        path: wasm_path,
        media_type: WASM_MEDIA.to_string(),
    });

    let migrations_path = artifact_dir.join(MIGRATIONS_BUNDLE);
    if migrations_path.is_file() {
        layers.push(PushLayer {
            path: migrations_path,
            media_type: MIGRATIONS_BUNDLE_MEDIA.to_string(),
        });
    }

    let operations_path = artifact_dir.join(OPERATIONS_BUNDLE);
    if operations_path.is_file() {
        layers.push(PushLayer {
            path: operations_path,
            media_type: OPERATIONS_BUNDLE_MEDIA.to_string(),
        });
    }

    let previews_path = module_root.join(PREVIEWS);
    if previews_path.is_file() {
        crate::manifest::ensure_inside(module_root, &previews_path)?;
        layers.push(PushLayer {
            path: previews_path,
            media_type: PREVIEWS_MEDIA.to_string(),
        });
    }

    let i18n_dir = module_root.join("i18n");
    if i18n_dir.is_dir() {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(&i18n_dir)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("json"))
            .collect();
        entries.sort();
        for path in entries {
            crate::manifest::ensure_inside(module_root, &path)?;
            layers.push(PushLayer {
                path,
                media_type: I18N_MEDIA.to_string(),
            });
        }
    }

    Ok(layers)
}

/// Host catalog is identified by localized `name` map without `manifestVersion`.
fn publish_layer_is_host_catalog_shape(path: &Path) -> Result<bool> {
    let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let root: serde_json::Value = serde_json::from_str(&raw).context("parse manifest json")?;
    if root.get("manifestVersion").is_some() {
        return Ok(false);
    }
    Ok(root
        .get("name")
        .and_then(|n| n.as_object())
        .is_some_and(|m| !m.is_empty()))
}

/// Converts push layers to `oci-distribution` image layers (reads bytes from disk).
pub fn layers_to_image_layers(layers: &[PushLayer]) -> Result<Vec<ImageLayer>> {
    let mut image_layers = Vec::with_capacity(layers.len());
    for layer in layers {
        let data = std::fs::read(&layer.path)
            .with_context(|| format!("read layer {}", layer.path.display()))?;
        image_layers.push(ImageLayer::new(data, layer.media_type.clone(), None));
    }
    Ok(image_layers)
}

/// Locates the wasm cargo just built for `module_id`.
///
/// Cargo names a library artifact after the *target*, not the package: `access-guide`
/// produces `access_guide.wasm`. Only hyphenated module ids differ, which is why this went
/// unnoticed — every single-word module resolves on the first candidate.
///
/// Shared with `commands::dev`, deliberately: the two paths diverged, `publish` grew a
/// directory scan that saved it and `dev` did not, so the same module built and deployed
/// from CI while failing on the author's machine.
/// The artifact a publish or a deploy ships — always the release one.
pub(crate) fn find_wasm_artifact(module_root: &Path, module_id: &str) -> Result<PathBuf> {
    find_wasm_artifact_in(module_root, module_id, "release")
}

/// The artifact of one profile.
///
/// `portaki build` without `--release` compiles debug and then has to look at what it just
/// built; looking under `release` made a plain `portaki build` end on an error after having
/// written every file it was asked for.
pub(crate) fn find_wasm_artifact_in(
    module_root: &Path,
    module_id: &str,
    profile: &str,
) -> Result<PathBuf> {
    let profile_dir = module_root.join(format!("target/wasm32-unknown-unknown/{profile}"));
    let candidates = [
        profile_dir.join(format!("{module_id}.wasm")),
        profile_dir.join(format!("{}.wasm", module_id.replace('-', "_"))),
    ];
    for candidate in &candidates {
        if candidate.exists() {
            return Ok(candidate.clone());
        }
    }

    if profile_dir.is_dir() {
        let mut wasm_files: Vec<PathBuf> = std::fs::read_dir(&profile_dir)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("wasm"))
            .collect();
        wasm_files.sort();
        if let Some(path) = wasm_files.into_iter().next() {
            return Ok(path);
        }
    }

    let rerun = if profile == "release" {
        "portaki build --release"
    } else {
        "portaki build"
    };
    anyhow::bail!(
        "no wasm artifact under {} — run {rerun} first",
        profile_dir.display()
    );
}

#[cfg(test)]
mod tests {
    /// Picking up an already published catalogue means reading id/version without a build: the
    /// publish-manifest does not exist until something has been compiled.
    #[test]
    fn the_linked_sdk_version_is_stamped_into_the_manifest() {
        let stamped = stamp_sdk_version(
            r#"{"id":"weather","version":"0.3.24"}"#,
            Some("2.1.1".into()),
        )
        .unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&stamped).unwrap();
        assert_eq!(parsed["requiresModuleSdk"], "2.1.1");
        assert_eq!(parsed["sdkVersion"], "2.1.1");
        assert_eq!(parsed["id"], "weather");
    }

    /// A manifest that announces another version lies about what was compiled.
    #[test]
    fn a_declared_version_that_disagrees_is_refused() {
        let err = stamp_sdk_version(
            r#"{"id":"weather","version":"0.3.24","requiresModuleSdk":"1.0.0"}"#,
            Some("2.1.1".into()),
        )
        .unwrap_err();

        assert!(err.to_string().contains("1.0.0"));
        assert!(err.to_string().contains("2.1.1"));
    }

    /// Declared and linked in agreement: nothing to report.
    #[test]
    fn a_declared_version_that_agrees_passes() {
        stamp_sdk_version(
            r#"{"id":"weather","requiresModuleSdk":"2.1.1"}"#,
            Some("2.1.1".into()),
        )
        .unwrap();
    }

    /// A module that does not depend on the SDK does not get a version invented for it.
    #[test]
    fn without_a_resolved_sdk_the_manifest_is_untouched() {
        let raw = r#"{"id":"weather","version":"0.3.24"}"#;

        assert_eq!(stamp_sdk_version(raw, None).unwrap(), raw);
    }

    /// The sandbox receives the stamped manifest, just as publication does.
    ///
    /// Without that stamp, `requiresModuleSdk` was missing from every module deployed by
    /// `portaki dev`, and the SDUI inspector refused to type — for every module, always. The
    /// message then advised "rebuild with portaki build", which writes elsewhere and changed
    /// nothing there.
    #[test]
    fn the_sandbox_manifest_carries_the_linked_sdk_version() {
        let raw = r#"{"id":"access-guide","version":"0.3.2"}"#;

        let stamped = stamp_sdk_version(raw, Some("2.1.1".to_string())).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&stamped).unwrap();
        assert_eq!(parsed["requiresModuleSdk"], "2.1.1");
        assert_eq!(parsed["id"], "access-guide");
    }

    #[test]
    fn source_coordinates_are_read_without_a_build() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("portaki.module.json"),
            r#"{"id":"weather","version":"0.3.24"}"#,
        )
        .unwrap();

        let coords = read_source_coordinates(dir.path()).unwrap();

        assert_eq!(coords.id, "weather");
        assert_eq!(coords.version, "0.3.24");
    }

    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn assemble_publish_manifest_copies_catalog_source() {
        let root = tempdir().unwrap();
        fs::write(
            root.path().join("portaki.module.json"),
            r#"{"id":"weather","version":"1.3.2"}"#,
        )
        .unwrap();
        let artifact = root.path().join("target/portaki");
        let path = assemble_publish_manifest(root.path(), &artifact).unwrap();
        assert_eq!(path, artifact.join(PUBLISH_MANIFEST));
        let raw = fs::read_to_string(&path).unwrap();
        assert!(raw.contains("\"version\":\"1.3.2\""));
    }

    #[test]
    fn assemble_publish_manifest_needs_no_hand_written_manifest() {
        let root = tempdir().unwrap();
        let artifact = root.path().join("target/portaki");
        fs::create_dir_all(&artifact).unwrap();
        fs::write(
            root.path().join("Cargo.toml"),
            "[package]\nname = \"weather\"\nversion = \"0.2.0\"\n",
        )
        .unwrap();
        fs::write(
            artifact.join("catalog.json"),
            r#"{"id":"weather","icon":"sun","permissions":["kv"]}"#,
        )
        .unwrap();
        fs::write(
            artifact.join("manifest.json"),
            r#"{"id":"weather","queries":[{"name":"current","fn":"current"}]}"#,
        )
        .unwrap();

        assemble_publish_manifest(root.path(), &artifact).unwrap();

        let published: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(artifact.join(PUBLISH_MANIFEST)).unwrap())
                .unwrap();
        assert_eq!(published["id"], "weather");
        assert_eq!(published["version"], "0.2.0");
        assert_eq!(published["icon"], "sun");
        assert_eq!(published["permissions"], serde_json::json!(["kv"]));
        assert_eq!(published["queries"][0]["name"], "current");
    }

    #[test]
    fn read_module_coordinates_reads_publish_manifest_only() {
        let dir = tempdir().unwrap();
        let artifact = dir.path().join("target/portaki");
        fs::create_dir_all(&artifact).unwrap();
        fs::write(
            dir.path().join("portaki.module.json"),
            r#"{"id":"stale","version":"0.0.1"}"#,
        )
        .unwrap();
        fs::write(
            artifact.join(PUBLISH_MANIFEST),
            r#"{"id":"weather","version":"0.2.0"}"#,
        )
        .unwrap();
        let coords = read_module_coordinates(dir.path(), &artifact).unwrap();
        assert_eq!(
            coords,
            ModuleCoordinates {
                id: "weather".to_string(),
                version: "0.2.0".to_string(),
            }
        );
    }

    #[test]
    fn collect_push_layers_uses_publish_manifest_not_repo_catalog() {
        let root = tempdir().unwrap();
        fs::write(
            root.path().join("portaki.module.json"),
            r#"{"id":"weather","version":"9.9.9"}"#,
        )
        .unwrap();
        let artifact = root.path().join("target/portaki");
        fs::create_dir_all(&artifact).unwrap();
        fs::write(
            artifact.join(PUBLISH_MANIFEST),
            r#"{"id":"weather","version":"0.1.0"}"#,
        )
        .unwrap();

        let wasm_dir = root.path().join("target/wasm32-unknown-unknown/release");
        fs::create_dir_all(&wasm_dir).unwrap();
        fs::write(wasm_dir.join("weather.wasm"), b"\0asm").unwrap();

        let layers = collect_push_layers(root.path(), &artifact).unwrap();
        assert_eq!(layers.len(), 2);
        assert_eq!(layers[0].path, artifact.join(PUBLISH_MANIFEST));
        assert_eq!(layers[0].media_type, MANIFEST_MEDIA);
    }

    #[test]
    fn collect_push_layers_carries_the_committed_previews() {
        let root = tempdir().unwrap();
        let artifact = root.path().join("target/portaki");
        fs::create_dir_all(&artifact).unwrap();
        fs::write(
            artifact.join(PUBLISH_MANIFEST),
            r#"{"id":"weather","version":"0.1.0"}"#,
        )
        .unwrap();
        let wasm_dir = root.path().join("target/wasm32-unknown-unknown/release");
        fs::create_dir_all(&wasm_dir).unwrap();
        fs::write(wasm_dir.join("weather.wasm"), b"\0asm").unwrap();
        fs::write(root.path().join(PREVIEWS), r#"{"surfaces":[]}"#).unwrap();

        let layers = collect_push_layers(root.path(), &artifact).unwrap();
        let previews = layers
            .iter()
            .find(|l| l.media_type == PREVIEWS_MEDIA)
            .unwrap();
        assert_eq!(previews.path, root.path().join(PREVIEWS));
    }

    /// Cargo names the artifact after the target: `access-guide` produces `access_guide.wasm`.
    /// `portaki dev` read the package name as it stood and failed on every module with a
    /// hyphenated name, while `publish` got by thanks to its directory scan.
    #[test]
    fn find_wasm_artifact_accepts_the_underscored_target_name() {
        let root = tempdir().unwrap();
        let wasm_dir = root.path().join("target/wasm32-unknown-unknown/release");
        fs::create_dir_all(&wasm_dir).unwrap();
        fs::write(wasm_dir.join("access_guide.wasm"), b"\0asm").unwrap();

        let found = find_wasm_artifact(root.path(), "access-guide").unwrap();
        assert_eq!(found, wasm_dir.join("access_guide.wasm"));
    }

    /// The exact name wins over the normalised one: a directory holding both must not depend
    /// on the order the entries are read in.
    #[test]
    fn find_wasm_artifact_prefers_the_exact_name() {
        let root = tempdir().unwrap();
        let wasm_dir = root.path().join("target/wasm32-unknown-unknown/release");
        fs::create_dir_all(&wasm_dir).unwrap();
        fs::write(wasm_dir.join("access_guide.wasm"), b"\0asm").unwrap();
        fs::write(wasm_dir.join("access-guide.wasm"), b"\0asm").unwrap();

        let found = find_wasm_artifact(root.path(), "access-guide").unwrap();
        assert_eq!(found, wasm_dir.join("access-guide.wasm"));
    }

    #[test]
    fn find_wasm_artifact_reports_the_directory_when_nothing_was_built() {
        let root = tempdir().unwrap();
        let error = find_wasm_artifact(root.path(), "access-guide").unwrap_err();
        assert!(error.to_string().contains("no wasm artifact"));
    }

    /// A plain `portaki build` compiles debug, and then has to find what it just built.
    #[test]
    fn find_wasm_artifact_reads_the_profile_it_is_given() {
        let root = tempdir().unwrap();
        let debug_dir = root.path().join("target/wasm32-unknown-unknown/debug");
        fs::create_dir_all(&debug_dir).unwrap();
        fs::write(debug_dir.join("access_guide.wasm"), b"\0asm").unwrap();

        let found = find_wasm_artifact_in(root.path(), "access-guide", "debug").unwrap();
        assert_eq!(found, debug_dir.join("access_guide.wasm"));
        // The release artifact is a different one, and is still missing.
        assert!(find_wasm_artifact(root.path(), "access-guide").is_err());
    }

    /// Telling someone to rerun with `--release` when they did not ask for release is noise.
    #[test]
    fn the_rerun_hint_matches_the_profile_that_was_missing() {
        let root = tempdir().unwrap();

        let debug = find_wasm_artifact_in(root.path(), "access-guide", "debug")
            .unwrap_err()
            .to_string();
        assert!(debug.contains("run portaki build first"), "{debug}");

        let release = find_wasm_artifact(root.path(), "access-guide")
            .unwrap_err()
            .to_string();
        assert!(
            release.contains("run portaki build --release first"),
            "{release}"
        );
    }

    #[test]
    fn collect_push_layers_includes_sdk_when_host_catalog_present() {
        let root = tempdir().unwrap();
        fs::write(
            root.path().join("portaki.module.json"),
            r#"{"id":"weather","version":"1.3.2","name":{"fr":"Météo","en":"Weather"},"description":{"fr":"d","en":"d"}}"#,
        )
        .unwrap();
        let artifact = root.path().join("target/portaki");
        fs::create_dir_all(&artifact).unwrap();
        fs::write(
            artifact.join(PUBLISH_MANIFEST),
            r#"{"id":"weather","version":"1.3.2","name":{"fr":"Météo","en":"Weather"},"description":{"fr":"d","en":"d"}}"#,
        )
        .unwrap();
        fs::write(
            artifact.join(SDK_MANIFEST),
            r#"{"manifestVersion":"1","id":"weather","version":"0.2.1","displayName":"module.name"}"#,
        )
        .unwrap();
        let wasm_dir = root.path().join("target/wasm32-unknown-unknown/release");
        fs::create_dir_all(&wasm_dir).unwrap();
        fs::write(wasm_dir.join("weather.wasm"), b"\0asm").unwrap();

        let layers = collect_push_layers(root.path(), &artifact).unwrap();
        assert_eq!(layers.len(), 3);
        assert_eq!(layers[0].media_type, MANIFEST_MEDIA);
        assert_eq!(layers[1].media_type, SDK_MANIFEST_MEDIA);
        assert_eq!(layers[1].path, artifact.join(SDK_MANIFEST));
    }

    #[test]
    fn collect_push_layers_sdk_only_single_manifest_layer() {
        let root = tempdir().unwrap();
        let artifact = root.path().join("target/portaki");
        fs::create_dir_all(&artifact).unwrap();
        fs::write(
            artifact.join(PUBLISH_MANIFEST),
            r#"{"manifestVersion":"1","id":"weather","version":"0.2.1"}"#,
        )
        .unwrap();
        let wasm_dir = root.path().join("target/wasm32-unknown-unknown/release");
        fs::create_dir_all(&wasm_dir).unwrap();
        fs::write(wasm_dir.join("weather.wasm"), b"\0asm").unwrap();

        let layers = collect_push_layers(root.path(), &artifact).unwrap();
        assert_eq!(layers.len(), 2);
        assert_eq!(layers[0].media_type, MANIFEST_MEDIA);
        assert!(layers
            .iter()
            .all(|layer| layer.media_type != SDK_MANIFEST_MEDIA));
    }

    /// A text linked elsewhere does not go out in a public layer — nor does a linked `i18n/` dir.
    #[cfg(unix)]
    #[test]
    fn collect_push_layers_refuses_what_links_outside_the_module() {
        let outside = tempdir().unwrap();
        fs::write(outside.path().join("secret.json"), "{}").unwrap();

        let linked_file = |root: &Path| {
            fs::create_dir_all(root.join("i18n")).unwrap();
            std::os::unix::fs::symlink(
                outside.path().join("secret.json"),
                root.join("i18n/fr.json"),
            )
            .unwrap();
        };
        let linked_dir = |root: &Path| {
            std::os::unix::fs::symlink(outside.path(), root.join("i18n")).unwrap();
        };
        for plant in [&linked_file as &dyn Fn(&Path), &linked_dir] {
            let root = tempdir().unwrap();
            let artifact = root.path().join("target/portaki");
            fs::create_dir_all(&artifact).unwrap();
            fs::write(
                artifact.join(PUBLISH_MANIFEST),
                r#"{"manifestVersion":"1","id":"weather","version":"0.2.1"}"#,
            )
            .unwrap();
            let wasm_dir = root.path().join("target/wasm32-unknown-unknown/release");
            fs::create_dir_all(&wasm_dir).unwrap();
            fs::write(wasm_dir.join("weather.wasm"), b"\0asm").unwrap();
            plant(root.path());

            let error = collect_push_layers(root.path(), &artifact).unwrap_err();
            assert!(error.to_string().contains("refusing to publish"), "{error}");
        }
    }
}

#[cfg(test)]
mod stamp_built_declarations_tests {
    use super::stamp_built_declarations as stamp_surfaces;

    const BUILT: &str = r#"{"id":"access-guide","surfaces":{"host":[{"id":"main","render_fn":"render_host_main"}],"guest":[]}}"#;

    /// The hand-written manifest does not say which symbol to call; the build does.
    #[test]
    fn carries_the_built_surfaces_into_the_uploaded_manifest() {
        let raw = r#"{"id":"access-guide","hostSurfaces":[{"pathSegment":"access-guide"}]}"#;

        let stamped = stamp_surfaces(raw, BUILT).expect("stamp");
        let value: serde_json::Value = serde_json::from_str(&stamped).expect("parse");

        assert_eq!(value["surfaces"]["host"][0]["id"], "main");
        // What the manifest already said is not erased: the pathSegment stays a piece of
        // navigation data, useful to the dashboard.
        assert_eq!(value["hostSurfaces"][0]["pathSegment"], "access-guide");
    }

    /// The sandbox's typed storage reads the entities from this manifest, and nowhere else.
    #[test]
    fn carries_the_built_entities_into_the_uploaded_manifest() {
        let raw = r#"{"id":"issue-report"}"#;
        let built =
            r#"{"id":"issue-report","entities":[{"name":"IssueReport","schema_version":2}]}"#;

        let stamped = stamp_surfaces(raw, built).expect("stamp");
        let value: serde_json::Value = serde_json::from_str(&stamped).expect("parse");

        assert_eq!(value["entities"][0]["name"], "IssueReport");
    }

    /// The code declares its emails; the hand-written manifest keeps its own and their texts.
    #[test]
    fn merges_the_built_emails_by_id_and_grants_the_permission() {
        let raw = r#"{"id":"issue-report","permissions":["repo"],"emails":[
            {"id":"submitted","description":{"fr":"Alerte"},"trigger":{"type":"x"}},
            {"id":"sync-failed","trigger":{"type":"onApplyFeeds"}}]}"#;
        let built = r#"{"id":"issue-report","emails":[
            {"id":"submitted","audience":"host","command":"submit","trigger":{"type":"moduleCommand"}},
            {"id":"resolved","audience":"guest","command":"resolve","trigger":{"type":"moduleCommand"}}]}"#;

        let stamped = stamp_surfaces(raw, built).expect("stamp");
        let value: serde_json::Value = serde_json::from_str(&stamped).expect("parse");

        let emails = value["emails"].as_array().expect("emails");
        assert_eq!(emails.len(), 3);
        assert_eq!(emails[0]["trigger"]["type"], "moduleCommand");
        assert_eq!(emails[0]["command"], "submit");
        assert_eq!(emails[0]["description"]["fr"], "Alerte");
        assert_eq!(emails[1]["id"], "sync-failed");
        assert_eq!(emails[2]["id"], "resolved");
        assert_eq!(value["permissions"], serde_json::json!(["repo", "email"]));

        // Stamped twice, nothing is duplicated.
        let again = stamp_surfaces(&stamped, built).expect("stamp");
        assert_eq!(again, stamped);
    }

    /// The orchestrator reads the connectors from the published manifest to lend a key.
    #[test]
    fn carries_the_built_connectors_into_the_uploaded_manifest() {
        let raw = r#"{"id":"local-guide","permissions":["connectors:tiqets"]}"#;
        let built = r#"{"id":"local-guide","connectors":{"builtin":[],"custom":[{"id":"tiqets","baseUrl":"https://api.tiqets.com","credentialProviderId":"tiqets","operations":[{"id":"nearby_products","method":"GET","path":"/v2/products"}]}]}}"#;

        let stamped = stamp_surfaces(raw, built).expect("stamp");
        let value: serde_json::Value = serde_json::from_str(&stamped).expect("parse");

        assert_eq!(
            value["connectors"]["custom"][0]["credentialProviderId"],
            "tiqets"
        );
        assert_eq!(
            value["connectors"]["custom"][0]["operations"][0]["path"],
            "/v2/products"
        );
        assert_eq!(
            value["permissions"],
            serde_json::json!(["connectors:tiqets"])
        );
    }

    /// A build that emits nothing must not prevent a deployment.
    #[test]
    fn leaves_the_manifest_alone_when_the_build_declares_no_surface() {
        let raw = r#"{"id":"access-guide"}"#;

        let stamped = stamp_surfaces(raw, r#"{"id":"access-guide"}"#).expect("stamp");

        assert_eq!(stamped, raw);
    }

    /// The operations only exist in the emitted manifest: without them, the sandbox can offer
    /// nothing but free-text entry of an operation name.
    #[test]
    fn carries_the_built_operations_into_the_uploaded_manifest() {
        let raw = r#"{"id":"ical-sync"}"#;
        let built = r#"{"id":"ical-sync","queries":[{"name":"listSources","fn":"list_sources"}],"commands":[{"name":"syncNow","fn":"sync_now"}]}"#;

        let stamped = stamp_surfaces(raw, built).expect("stamp");
        let value: serde_json::Value = serde_json::from_str(&stamped).expect("parse");

        assert_eq!(value["queries"][0]["name"], "listSources");
        assert_eq!(value["commands"][0]["fn"], "sync_now");
        // A key the build did not emit is not invented.
        assert!(value.get("surfaces").is_none());
    }

    /// An empty list is an answer: it says the build exposes nothing, and it travels.
    #[test]
    fn carries_an_empty_operation_list_as_such() {
        let stamped = stamp_surfaces(r#"{"id":"m"}"#, r#"{"id":"m","queries":[],"commands":[]}"#)
            .expect("stamp");
        let value: serde_json::Value = serde_json::from_str(&stamped).expect("parse");

        assert_eq!(value["queries"], serde_json::json!([]));
        assert_eq!(value["commands"], serde_json::json!([]));
    }

    /// The emitted surfaces are authoritative: they describe the bytes that are going to run.
    #[test]
    fn built_surfaces_win_over_anything_already_declared() {
        let raw = r#"{"surfaces":{"host":[{"id":"stale"}]}}"#;

        let stamped = stamp_surfaces(raw, BUILT).expect("stamp");
        let value: serde_json::Value = serde_json::from_str(&stamped).expect("parse");

        assert_eq!(value["surfaces"]["host"][0]["id"], "main");
    }
}

#[cfg(test)]
mod assemble_publish_manifest_tests {
    use super::{assemble_publish_manifest, publish_manifest_path};
    use std::fs;

    fn module_with(catalogue: &str, built: Option<&str>) -> tempfile::TempDir {
        let module = tempfile::tempdir().expect("tempdir");
        let artifacts = module.path().join("target/portaki");
        fs::create_dir_all(&artifacts).expect("artifact dir");
        fs::write(module.path().join("portaki.module.json"), catalogue).expect("catalogue");
        if let Some(built) = built {
            fs::write(artifacts.join("manifest.json"), built).expect("built manifest");
        }
        module
    }

    /// The published manifest is the one the platform reads. The operations only exist in the
    /// build's output — a wasm module exports only `portaki_query` and cannot say what it
    /// serves. Without carrying them over, the platform has to query them all blindly.
    #[test]
    fn the_published_manifest_carries_the_built_operations() {
        let module = module_with(
            r#"{"id":"local-guide","hostSurfaces":[{"pathSegment":"local-guide"}]}"#,
            Some(
                r#"{"id":"local-guide","queries":[{"name":"mapMarkers","fn":"map_markers"}],"commands":[]}"#,
            ),
        );
        let artifacts = module.path().join("target/portaki");

        assemble_publish_manifest(module.path(), &artifacts).expect("assemble");

        let raw = fs::read_to_string(publish_manifest_path(&artifacts)).expect("read");
        let value: serde_json::Value = serde_json::from_str(&raw).expect("parse");
        assert_eq!(value["queries"][0]["name"], "mapMarkers");
        // An empty list is an answer: the module mutates nothing, and it says so.
        assert_eq!(value["commands"], serde_json::json!([]));
        // What the catalogue manifest already carried survives: the pathSegment stays a
        // piece of the dashboard's navigation data.
        assert_eq!(value["hostSurfaces"][0]["pathSegment"], "local-guide");
    }

    /// A module with no build output stays publishable: nothing is invented.
    #[test]
    fn a_module_without_build_output_publishes_unchanged() {
        let module = module_with(r#"{"id":"m"}"#, None);
        let artifacts = module.path().join("target/portaki");

        assemble_publish_manifest(module.path(), &artifacts).expect("assemble");

        let raw = fs::read_to_string(publish_manifest_path(&artifacts)).expect("read");
        assert!(!raw.contains("queries"), "{raw}");
    }
}
