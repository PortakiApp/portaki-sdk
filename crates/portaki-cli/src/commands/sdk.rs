//! `portaki sdk upgrade` — move a module to another SDK version, and prove nothing broke.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use serde::Deserialize;
use toml_edit::{DocumentMut, Item, Value};

use crate::commands::dev;
use crate::ui;

/// Les crates qui montent ensemble. Le SDK les publie à la même version, et en mélanger deux
/// ferait compiler un module contre des macros d'une génération et un runtime d'une autre.
const SDK_FAMILY: [&str; 4] = [
    "portaki-sdk",
    "portaki-sdk-macros",
    "portaki-connectors",
    "portaki-test-utils",
];

#[derive(Debug, Parser)]
/// Arguments for `portaki sdk`.
pub struct SdkArgs {
    #[command(subcommand)]
    pub command: SdkCommand,
}

#[derive(Debug, Subcommand)]
pub enum SdkCommand {
    /// Move the module to another SDK version, then check it still builds, passes and renders.
    Upgrade(UpgradeArgs),
}

#[derive(Debug, Parser)]
/// Arguments for `portaki sdk upgrade`.
pub struct UpgradeArgs {
    /// Target version. Defaults to the latest stable release the platform serves.
    #[arg(long)]
    pub to: Option<String>,
    /// Skip the sandbox render comparison (build, tests and lint still run).
    #[arg(long)]
    pub no_render: bool,
    /// Treat a changed render as a failure, not only a surface that stopped rendering.
    #[arg(long)]
    pub strict: bool,
    /// Platform URL (defaults like `portaki dev`: PORTAKI_DEV_URL, PORTAKI_API_URL, production).
    #[arg(long)]
    pub url: Option<String>,
}

pub async fn run(args: SdkArgs) -> Result<()> {
    match args.command {
        SdkCommand::Upgrade(upgrade) => run_upgrade(upgrade).await,
    }
}

// ─── Où la version est déclarée ──────────────────────────────────────────────

/// Le fichier qui porte l'exigence de version, et ce qu'y changer touche.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declaration {
    pub file: PathBuf,
    /// L'exigence vient de `[workspace.dependencies]` : la changer fait monter tous les membres.
    pub inherited: bool,
}

/// Where `portaki-sdk` is required from, read in the module's own `Cargo.toml`.
///
/// `portaki-sdk = { workspace = true }` sends the question to the workspace root: that is
/// where the number lives, and changing it moves every member — which the command must say
/// before doing it, not after.
pub fn locate_declaration(
    module_toml: &str,
    module_root: &Path,
    workspace_root: &Path,
) -> Result<Declaration> {
    let doc: DocumentMut = module_toml.parse().context("parse Cargo.toml")?;
    let sdk = doc
        .get("dependencies")
        .and_then(|deps| deps.get("portaki-sdk"))
        .context("this crate does not depend on portaki-sdk")?;
    let inherited = sdk
        .as_table_like()
        .and_then(|table| table.get("workspace"))
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    Ok(Declaration {
        file: if inherited {
            workspace_root.join("Cargo.toml")
        } else {
            module_root.join("Cargo.toml")
        },
        inherited,
    })
}

/// Rewrites every SDK-family requirement to `target`, keeping the file as it was otherwise.
///
/// Comments, ordering and other keys survive: this is the author's `Cargo.toml`, not a file the
/// CLI owns. An inline table keeps its other keys (`features`, `default-features`…); only its
/// `version` moves. An entry inherited from the workspace is left alone — it has no number here.
///
/// @returns the new file and the crates it changed, in family order
pub fn bump_requirements(
    toml: &str,
    inherited: bool,
    target: &str,
) -> Result<(String, Vec<String>)> {
    let mut doc: DocumentMut = toml.parse().context("parse Cargo.toml")?;
    let sections: Vec<Vec<&str>> = if inherited {
        vec![vec!["workspace", "dependencies"]]
    } else {
        vec![
            vec!["dependencies"],
            vec!["dev-dependencies"],
            vec!["build-dependencies"],
        ]
    };
    let mut changed = Vec::new();
    for path in sections {
        let Some(table) = walk_mut(doc.as_item_mut(), &path) else {
            continue;
        };
        for name in SDK_FAMILY {
            let Some(entry) = table.get_mut(name) else {
                continue;
            };
            if set_version(entry, target) && !changed.iter().any(|c: &String| c == name) {
                changed.push(name.to_string());
            }
        }
    }
    Ok((doc.to_string(), changed))
}

fn walk_mut<'a>(item: &'a mut Item, path: &[&str]) -> Option<&'a mut dyn toml_edit::TableLike> {
    let mut current = item;
    for key in path {
        current = current.as_table_like_mut()?.get_mut(key)?;
    }
    current.as_table_like_mut()
}

fn set_version(entry: &mut Item, target: &str) -> bool {
    if entry.as_value().is_some_and(Value::is_str) {
        return entry
            .as_value_mut()
            .is_some_and(|value| replace_requirement(value, target));
    }
    let Some(table) = entry.as_table_like_mut() else {
        return false;
    };
    if table.get("workspace").and_then(|v| v.as_bool()) == Some(true) {
        return false;
    }
    table
        .get_mut("version")
        .and_then(Item::as_value_mut)
        .is_some_and(|value| replace_requirement(value, target))
}

/// Swaps the number, keeps the operator: `=2.2.0` pinned the SDK on purpose, and must stay
/// pinned at the new version — dropping the `=` would silently turn it into a caret range.
fn replace_requirement(value: &mut Value, target: &str) -> bool {
    let Some(current) = value.as_str() else {
        return false;
    };
    let operator: String = current
        .chars()
        .take_while(|c| matches!(c, '=' | '^' | '~' | ' '))
        .collect();
    let decor = value.decor().clone();
    *value = Value::from(format!("{}{target}", operator.trim_end()));
    *value.decor_mut() = decor;
    true
}

// ─── Versions ────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SdkRelease {
    pub version: String,
    pub channel: String,
    #[serde(default = "yes")]
    pub supported: bool,
}

fn yes() -> bool {
    true
}

fn parse_version(version: &str) -> Option<(u64, u64, u64)> {
    let core = version.trim().split(['-', '+']).next()?;
    let mut parts = core.split('.').map(|part| part.parse::<u64>().ok());
    Some((
        parts.next()??,
        parts.next().flatten().unwrap_or(0),
        parts.next().flatten().unwrap_or(0),
    ))
}

/// The latest supported stable release — computed, not read at the head of the list, whose
/// order is promised nowhere. Numeric: as text, `10.0` would sort before `2.0`.
pub fn latest_stable(releases: &[SdkRelease]) -> Option<String> {
    releases
        .iter()
        .filter(|release| release.channel == "stable" && release.supported)
        .filter_map(|release| parse_version(&release.version).map(|v| (v, &release.version)))
        .max_by_key(|(parsed, _)| *parsed)
        .map(|(_, version)| version.clone())
}

/// What to say when the lock resolved something other than the version asked for.
///
/// A plain `"3.0.1"` requirement is a caret: once 3.1.0 is out, it resolves 3.1.0. That is
/// usually what one wants — but not what one typed, and the commit message must not claim
/// otherwise.
pub fn resolution_note(target: &str, resolved: &str) -> Option<String> {
    let (wanted, got) = (parse_version(target)?, parse_version(resolved)?);
    if wanted == got {
        return None;
    }
    Some(if got > wanted {
        format!(
            "the requirement {target} resolved {resolved}, the newest compatible release — \
             write ={target} to hold {target} instead"
        )
    } else {
        format!("the requirement {target} resolved {resolved}, below what was asked")
    })
}

// ─── Rendus ──────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeclaredSurface {
    id: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Rendered {
    pub rendered: bool,
    #[serde(default)]
    pub tree: String,
    #[serde(default)]
    pub error_code: String,
}

/// Ce qu'une montée de version a fait à une surface.
#[derive(Debug, PartialEq, Eq)]
pub enum RenderOutcome {
    Same,
    /// Elle rend toujours, autrement — pas forcément une faute : une primitive peut avoir
    /// gagné un champ. `--strict` en fait une.
    Changed {
        before: usize,
        after: usize,
    },
    /// Elle rendait, elle ne rend plus. C'est la casse que la commande existe pour trouver.
    Broke {
        error: String,
    },
    /// Elle ne rendait déjà pas — la montée n'y est pour rien.
    StillFailing,
    /// Elle ne rendait pas, elle rend — rare, et bon à savoir.
    Fixed,
}

pub fn compare_render(before: &Rendered, after: &Rendered) -> RenderOutcome {
    match (before.rendered, after.rendered) {
        (true, false) => RenderOutcome::Broke {
            error: if after.error_code.is_empty() {
                "sans motif".into()
            } else {
                after.error_code.clone()
            },
        },
        (false, false) => RenderOutcome::StillFailing,
        (false, true) => RenderOutcome::Fixed,
        (true, true) => {
            let parse = |raw: &str| serde_json::from_str::<serde_json::Value>(raw).ok();
            match (parse(&before.tree), parse(&after.tree)) {
                // L'égalité de serde_json ignore l'ordre des clés : un objet réordonné n'a pas
                // changé de rendu.
                (Some(a), Some(b)) if a == b => RenderOutcome::Same,
                (Some(a), Some(b)) => RenderOutcome::Changed {
                    before: nodes(&a),
                    after: nodes(&b),
                },
                _ if before.tree == after.tree => RenderOutcome::Same,
                _ => RenderOutcome::Changed {
                    before: 0,
                    after: 0,
                },
            }
        }
    }
}

/// The SDUI nodes of a tree — objects that carry a `type`. A cheap size to report a change by.
fn nodes(value: &serde_json::Value) -> usize {
    match value {
        serde_json::Value::Object(map) => {
            usize::from(map.contains_key("type")) + map.values().map(nodes).sum::<usize>()
        }
        serde_json::Value::Array(items) => items.iter().map(nodes).sum(),
        _ => 0,
    }
}

// ─── L'enchaînement ──────────────────────────────────────────────────────────

struct Backup {
    files: Vec<(PathBuf, Option<String>)>,
}

impl Backup {
    fn take(paths: &[PathBuf]) -> Self {
        Self {
            files: paths
                .iter()
                .map(|p| (p.clone(), std::fs::read_to_string(p).ok()))
                .collect(),
        }
    }

    fn restore(&self) -> Result<()> {
        for (path, content) in &self.files {
            match content {
                Some(content) => std::fs::write(path, content)
                    .with_context(|| format!("restore {}", path.display()))?,
                None => {
                    let _ = std::fs::remove_file(path);
                }
            }
        }
        Ok(())
    }
}

struct Sandbox {
    base_url: String,
    module_id: String,
    token: String,
    session: crate::dev_session::DevSession,
}

impl Sandbox {
    async fn deploy_and_render(
        &mut self,
        module_root: &Path,
    ) -> Result<BTreeMap<String, Rendered>> {
        let wasm_path = crate::oci::pack::find_wasm_artifact(module_root, &self.module_id)?;
        let wasm =
            std::fs::read(&wasm_path).with_context(|| format!("read {}", wasm_path.display()))?;
        let manifest = dev::sandbox_manifest(module_root)?;
        let deploying = ui::step(format!("deploying {} to the sandbox", self.module_id));
        let deployed = match dev::deploy(
            &self.base_url,
            &self.module_id,
            &self.token,
            &wasm,
            &manifest,
            self.session.session_id(),
        )
        .await
        {
            Ok(deployed) => deployed,
            Err(_) => {
                self.token = dev::reauthenticate().await?;
                dev::deploy(
                    &self.base_url,
                    &self.module_id,
                    &self.token,
                    &wasm,
                    &manifest,
                    self.session.session_id(),
                )
                .await?
            }
        };
        deploying.done(format!("deployed {}", dev::short(&deployed.digest)));

        let client = reqwest::Client::new();
        let surfaces: Vec<DeclaredSurface> = dev::read_json(
            client
                .get(format!(
                    "{}/dev/v1/modules/{}/surfaces",
                    self.base_url, self.module_id
                ))
                .bearer_auth(&self.token)
                .send()
                .await
                .context("list the declared surfaces")?,
        )
        .await?;
        let mut rendered = BTreeMap::new();
        for surface in surfaces {
            let rendering = ui::step(format!("rendering {}", surface.id));
            let result: Rendered = dev::read_json(
                client
                    .post(format!(
                        "{}/dev/v1/modules/{}/surfaces/{}/render",
                        self.base_url, self.module_id, surface.id
                    ))
                    .bearer_auth(&self.token)
                    .send()
                    .await
                    .with_context(|| format!("render {}", surface.id))?,
            )
            .await?;
            if result.rendered {
                rendering.done(format!("{} rendered", surface.id));
            } else {
                rendering.done(format!(
                    "{} did not render ({})",
                    surface.id, result.error_code
                ));
            }
            rendered.insert(surface.id, result);
        }
        Ok(rendered)
    }

    /// Les contrôles de conformité en échec — ceux qui verrouilleraient une publication stable.
    async fn failing_checks(&self) -> Result<Vec<String>> {
        #[derive(Deserialize)]
        struct Report {
            checks: Vec<Check>,
        }
        #[derive(Deserialize)]
        struct Check {
            id: String,
            status: String,
            #[serde(default)]
            detail: String,
        }
        let report: Report = dev::read_json(
            reqwest::Client::new()
                .get(format!(
                    "{}/dev/v1/modules/{}/conformance",
                    self.base_url, self.module_id
                ))
                .bearer_auth(&self.token)
                .send()
                .await
                .context("read the conformance checklist")?,
        )
        .await?;
        Ok(report
            .checks
            .into_iter()
            .filter(|check| check.status == "FAIL")
            .map(|check| format!("{} — {}", check.id, check.detail))
            .collect())
    }
}

#[derive(Debug, Deserialize)]
struct Metadata {
    workspace_root: PathBuf,
    packages: Vec<MetadataPackage>,
    workspace_members: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct MetadataPackage {
    name: String,
    version: String,
}

fn cargo_metadata(module_root: &Path) -> Result<Metadata> {
    let output = Command::new("cargo")
        .args(["metadata", "--format-version", "1"])
        .current_dir(module_root)
        .output()
        .context("run cargo metadata")?;
    if !output.status.success() {
        bail!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    serde_json::from_slice(&output.stdout).context("parse cargo metadata")
}

fn cargo(module_root: &Path, label: &str, args: &[&str]) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.current_dir(module_root).args(args);
    ui::command(label, &mut cmd).with_context(|| format!("cargo {}", args.join(" ")))
}

async fn run_upgrade(args: UpgradeArgs) -> Result<()> {
    ui::header(
        "portaki sdk upgrade",
        "Move to another SDK version — then build, test, lint and render to prove nothing broke.",
    );
    let module_root = std::env::current_dir().context("current_dir")?;
    let module_id = dev::read_module_id(&module_root)?;
    let base_url = dev::resolve_base_url(
        args.url.as_deref(),
        std::env::var("PORTAKI_DEV_URL").ok().as_deref(),
        std::env::var("PORTAKI_API_URL").ok().as_deref(),
    );

    let metadata = cargo_metadata(&module_root)?;
    let current = metadata
        .packages
        .iter()
        .find(|package| package.name == "portaki-sdk")
        .map(|package| package.version.clone())
        .context("portaki-sdk is not in the dependency graph")?;

    let target = match &args.to {
        Some(version) => version.trim().to_string(),
        None => {
            let reading = ui::step("reading the SDK releases the platform serves");
            let releases: Vec<SdkRelease> = dev::read_json(
                reqwest::Client::new()
                    .get(format!("{base_url}/registry/v1/sdk-releases"))
                    .send()
                    .await
                    .context("read the SDK releases")?,
            )
            .await?;
            let latest =
                latest_stable(&releases).context("the platform lists no stable SDK release")?;
            reading.done(format!("latest stable: {latest}"));
            latest
        }
    };
    ui::field("current", &current);
    ui::field("target", &target);
    if parse_version(&current) == parse_version(&target) {
        ui::success(format!(
            "{module_id} already resolves portaki-sdk {current}"
        ));
        return Ok(());
    }

    let module_toml =
        std::fs::read_to_string(module_root.join("Cargo.toml")).context("read Cargo.toml")?;
    let declaration = locate_declaration(&module_toml, &module_root, &metadata.workspace_root)?;
    ui::field("declared in", declaration.file.display());
    if declaration.inherited {
        ui::warn(format!(
            "{module_id} inherits the SDK from the workspace: the {} members move together, and all of them are built and tested",
            metadata.workspace_members.len()
        ));
    }

    let mut sandbox = if args.no_render {
        ui::skipped("render comparison skipped (--no-render)");
        None
    } else {
        let token = crate::auth::access_token()
            .context("sign in with `portaki login`, or pass --no-render")?;
        let session = crate::dev_session::start(&base_url, &module_id, &token).await?;
        Some(Sandbox {
            base_url: base_url.clone(),
            module_id: module_id.clone(),
            token,
            session,
        })
    };

    let baseline = match sandbox.as_mut() {
        Some(sandbox) => {
            ui::section(&format!("before — portaki-sdk {current}"));
            dev::build(&module_root)?;
            crate::commands::build::refresh_outputs(&module_root)?;
            Some(sandbox.deploy_and_render(&module_root).await?)
        }
        None => None,
    };

    ui::section(&format!("after — portaki-sdk {target}"));
    let lock = metadata.workspace_root.join("Cargo.lock");
    let backup = Backup::take(&[declaration.file.clone(), lock]);

    let outcome = upgrade_and_verify(
        &args,
        &module_root,
        &declaration,
        &metadata,
        &target,
        sandbox.as_mut(),
        baseline.as_ref(),
    )
    .await;

    if let Some(sandbox) = &sandbox {
        sandbox.session.release().now().await;
    }
    match outcome {
        Ok(resolved) => {
            ui::blank();
            // La version résolue, pas la cible : `"3.0.1"` est un caret, et résout 3.1.0 dès
            // que 3.1.0 existe. Annoncer la cible aurait fait committer un message faux.
            ui::success(format!("{module_id} is on portaki-sdk {resolved}"));
            // Hérité, le changement est à la racine : un `git diff` lancé depuis le module ne
            // montrerait que son propre Cargo.toml, inchangé.
            let review = if declaration.inherited {
                format!(
                    "git -C {} diff Cargo.toml Cargo.lock",
                    metadata.workspace_root.display()
                )
            } else {
                "git diff Cargo.toml Cargo.lock".to_string()
            };
            ui::next(&[
                ("review", &review),
                (
                    "commit",
                    &format!("chore(deps): bump portaki-sdk to {resolved}"),
                ),
            ]);
            Ok(())
        }
        Err(failure) => {
            backup.restore()?;
            ui::blank();
            ui::failure("the upgrade broke something — Cargo.toml and Cargo.lock are restored");
            if sandbox.is_some() {
                ui::advice("the sandbox now holds the attempted build: run `portaki dev` to put yours back");
            }
            Err(failure)
        }
    }
}

async fn upgrade_and_verify(
    args: &UpgradeArgs,
    module_root: &Path,
    declaration: &Declaration,
    metadata: &Metadata,
    target: &str,
    sandbox: Option<&mut Sandbox>,
    baseline: Option<&BTreeMap<String, Rendered>>,
) -> Result<String> {
    let original = std::fs::read_to_string(&declaration.file)
        .with_context(|| format!("read {}", declaration.file.display()))?;
    let (bumped, changed) = bump_requirements(&original, declaration.inherited, target)?;
    if changed.is_empty() {
        bail!(
            "no SDK requirement with a version number in {}",
            declaration.file.display()
        );
    }
    std::fs::write(&declaration.file, bumped)
        .with_context(|| format!("write {}", declaration.file.display()))?;
    ui::wrote("requirement", format!("{} → {target}", changed.join(", ")));

    // Seulement les crates que le graphe résout : `cargo update -p` refuse un nom inconnu.
    let resolved: Vec<&str> = SDK_FAMILY
        .into_iter()
        .filter(|name| {
            metadata
                .packages
                .iter()
                .any(|package| package.name == *name)
        })
        .collect();
    let mut update = vec!["update"];
    for name in &resolved {
        update.push("-p");
        update.push(name);
    }
    cargo(module_root, "resolving the new version", &update)?;
    let resolved_version = cargo_metadata(module_root)?
        .packages
        .into_iter()
        .find(|package| package.name == "portaki-sdk")
        .map(|package| package.version)
        .context("portaki-sdk left the dependency graph")?;
    ui::field("resolved", &resolved_version);
    if let Some(note) = resolution_note(target, &resolved_version) {
        ui::warn(note);
    }

    let scope: &[&str] = if declaration.inherited {
        &["--workspace"]
    } else {
        &[]
    };
    let build: Vec<&str> = ["build", "--release", "--target", "wasm32-unknown-unknown"]
        .into_iter()
        .chain(scope.iter().copied())
        .collect();
    cargo(
        module_root,
        "compiling wasm32-unknown-unknown (release)",
        &build,
    )?;
    crate::commands::build::refresh_outputs(module_root)?;
    let test: Vec<&str> = ["test"].into_iter().chain(scope.iter().copied()).collect();
    cargo(module_root, "running the tests", &test)?;
    crate::commands::lint::run(crate::commands::lint::LintArgs { manifest: None })?;

    let (Some(sandbox), Some(baseline)) = (sandbox, baseline) else {
        return Ok(resolved_version);
    };
    let after = sandbox.deploy_and_render(module_root).await?;

    let mut broken = Vec::new();
    let mut changed_renders = Vec::new();
    for (id, before) in baseline {
        let Some(now) = after.get(id) else {
            broken.push(format!("{id} is no longer declared"));
            continue;
        };
        match compare_render(before, now) {
            RenderOutcome::Same => ui::detail(format!("{id}: same render")),
            RenderOutcome::Changed { before, after } => {
                ui::warn(format!(
                    "{id}: renders differently ({before} → {after} nodes)"
                ));
                changed_renders.push(id.clone());
            }
            RenderOutcome::Broke { error } => {
                broken.push(format!("{id} stopped rendering: {error}"))
            }
            RenderOutcome::StillFailing => {
                ui::detail(format!("{id}: did not render before either"))
            }
            RenderOutcome::Fixed => ui::detail(format!("{id}: renders now, it did not before")),
        }
    }
    let failing = sandbox.failing_checks().await?;
    for check in &failing {
        broken.push(format!("conformance: {check}"));
    }
    if args.strict {
        broken.extend(
            changed_renders
                .iter()
                .map(|id| format!("{id} renders differently (--strict)")),
        );
    }
    if !broken.is_empty() {
        for line in &broken {
            ui::failure(line);
        }
        bail!("{} regression(s) after the upgrade", broken.len());
    }
    Ok(resolved_version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_workspace_inherited_sdk_points_at_the_workspace_root() {
        let module = "[package]\nname = \"ical-sync\"\n\n[dependencies]\nportaki-sdk = { workspace = true }\n";
        let declaration =
            locate_declaration(module, Path::new("/m/ical"), Path::new("/m")).unwrap();

        assert_eq!(
            declaration,
            Declaration {
                file: PathBuf::from("/m/Cargo.toml"),
                inherited: true
            }
        );
    }

    #[test]
    fn an_own_requirement_points_at_the_module() {
        let module = "[dependencies]\nportaki-sdk = \"2.4.0\"\n";
        let declaration =
            locate_declaration(module, Path::new("/m/ical"), Path::new("/m")).unwrap();

        assert!(!declaration.inherited);
        assert_eq!(declaration.file, PathBuf::from("/m/ical/Cargo.toml"));
    }

    /// The workspace root of portaki-modules, as it is.
    #[test]
    fn the_whole_family_moves_and_the_rest_of_the_file_stays() {
        let root = "[workspace]\nmembers = [\"modules/*\"]\n\n[workspace.dependencies]\n# the SDK family moves together\nportaki-sdk = \"2.2.0\"\nportaki-sdk-macros = \"2.2.0\"\nportaki-connectors = \"2.2.0\"\nportaki-test-utils = \"2.2.0\"\nserde = { version = \"1\", features = [\"derive\"] }\n";

        let (bumped, changed) = bump_requirements(root, true, "3.0.1").unwrap();

        assert_eq!(
            changed,
            vec![
                "portaki-sdk",
                "portaki-sdk-macros",
                "portaki-connectors",
                "portaki-test-utils"
            ]
        );
        assert!(bumped.contains("portaki-sdk = \"3.0.1\""));
        assert!(bumped.contains("# the SDK family moves together"));
        assert!(bumped.contains("serde = { version = \"1\", features = [\"derive\"] }"));
    }

    #[test]
    fn an_inline_table_keeps_its_other_keys() {
        let module = "[dependencies]\nportaki-sdk = { version = \"2.4.0\", default-features = false }\n\n[dev-dependencies]\nportaki-test-utils = \"2.4.0\"\n";

        let (bumped, changed) = bump_requirements(module, false, "3.0.1").unwrap();

        assert!(bumped.contains("portaki-sdk = { version = \"3.0.1\", default-features = false }"));
        assert!(bumped.contains("portaki-test-utils = \"3.0.1\""));
        assert_eq!(changed, vec!["portaki-sdk", "portaki-test-utils"]);
    }

    /// No number here: the command must not invent one next to `workspace = true`.
    #[test]
    fn an_inherited_entry_is_left_alone_in_the_module() {
        let module = "[dependencies]\nportaki-sdk = { workspace = true }\n";

        let (bumped, changed) = bump_requirements(module, false, "3.0.1").unwrap();

        assert!(changed.is_empty());
        assert_eq!(bumped, module);
    }

    #[test]
    fn a_pinned_requirement_stays_pinned() {
        let module = "[dependencies]\nportaki-sdk = \"=2.4.0\"\nportaki-sdk-macros = { version = \"~2.4\" }\n";

        let (bumped, _) = bump_requirements(module, false, "3.0.1").unwrap();

        assert!(bumped.contains("portaki-sdk = \"=3.0.1\""));
        assert!(bumped.contains("portaki-sdk-macros = { version = \"~3.0.1\" }"));
    }

    /// A caret requirement resolves the newest compatible release — say so, don't claim the target.
    #[test]
    fn a_newer_compatible_resolution_is_reported() {
        let note = resolution_note("3.0.1", "3.1.0").unwrap();

        assert!(note.contains("resolved 3.1.0"));
        assert!(note.contains("=3.0.1"));
        assert_eq!(resolution_note("3.1.0", "3.1.0"), None);
    }

    fn release(version: &str, channel: &str, supported: bool) -> SdkRelease {
        SdkRelease {
            version: version.into(),
            channel: channel.into(),
            supported,
        }
    }

    #[test]
    fn the_latest_stable_is_computed_numerically() {
        let releases = [
            release("2.6.0", "stable", true),
            release("10.0.0", "stable", true),
            release("3.0.1", "stable", true),
            release("11.0.0-rc.1", "rc", true),
            release("12.0.0", "stable", false),
        ];

        assert_eq!(latest_stable(&releases).as_deref(), Some("10.0.0"));
    }

    fn render(rendered: bool, tree: &str, error: &str) -> Rendered {
        Rendered {
            rendered,
            tree: tree.into(),
            error_code: error.into(),
        }
    }

    #[test]
    fn a_surface_that_stops_rendering_is_a_break() {
        assert_eq!(
            compare_render(
                &render(true, r#"{"type":"Text"}"#, ""),
                &render(false, "", "wasm_trap")
            ),
            RenderOutcome::Broke {
                error: "wasm_trap".into()
            }
        );
    }

    /// Key order is not a render change.
    #[test]
    fn a_reordered_tree_is_the_same_render() {
        assert_eq!(
            compare_render(
                &render(true, r#"{"type":"Text","content":"a"}"#, ""),
                &render(true, r#"{"content":"a","type":"Text"}"#, ""),
            ),
            RenderOutcome::Same
        );
    }

    #[test]
    fn a_different_tree_is_reported_with_its_size() {
        assert_eq!(
            compare_render(
                &render(true, r#"{"type":"Stack","children":[{"type":"Text"}]}"#, ""),
                &render(
                    true,
                    r#"{"type":"Stack","children":[{"type":"Text"},{"type":"Button"}]}"#,
                    ""
                ),
            ),
            RenderOutcome::Changed {
                before: 2,
                after: 3
            }
        );
    }

    #[test]
    fn a_surface_that_already_failed_is_not_blamed_on_the_upgrade() {
        assert_eq!(
            compare_render(&render(false, "", "x"), &render(false, "", "x")),
            RenderOutcome::StillFailing
        );
    }
}
