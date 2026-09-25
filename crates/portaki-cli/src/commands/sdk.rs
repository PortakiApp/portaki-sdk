//! `portaki sdk upgrade` — move a module to another SDK version, and prove nothing broke.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use serde::Deserialize;
use toml_edit::{DocumentMut, Item, Value};

use crate::commands::dev;
use crate::{ui, workspace};

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
    /// Run every check against the new version, then put every file it changed back.
    #[arg(long)]
    pub dry_run: bool,
}

pub async fn run(args: SdkArgs) -> Result<()> {
    match args.command {
        SdkCommand::Upgrade(upgrade) => run_upgrade(upgrade).await,
    }
}

// ─── Où la version est déclarée ──────────────────────────────────────────────

/// Les fichiers qui portent l'exigence de version, et ce qu'y changer touche.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declaration {
    pub files: Vec<PathBuf>,
    /// Le workspace fixe la version pour tous : hérité (`workspace = true`), ou épinglé à la
    /// racine et recopié dans chaque module. Monter l'un sans les autres casserait le dépôt.
    pub workspace_wide: bool,
}

/// Where `portaki-sdk` is required from, read in the module's own `Cargo.toml`.
///
/// Three layouts. `portaki-sdk = { workspace = true }` sends the question to the workspace root.
/// A root that pins the family in `[workspace.dependencies]` while each module writes its own
/// number (portaki-modules: release-please attributes a bump per module path) moves as one —
/// the root and every member that declares a number. A lone module moves alone.
///
/// @param workspace_toml the root `Cargo.toml`, when the module sits in a workspace
/// @param members each member's root and `Cargo.toml`
pub fn locate_declaration(
    module_toml: &str,
    module_root: &Path,
    workspace_root: &Path,
    workspace_toml: Option<&str>,
    members: &[(PathBuf, String)],
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
    let root_pins = workspace_toml.is_some_and(pins_the_family);
    if !inherited && !root_pins {
        return Ok(Declaration {
            files: vec![module_root.join("Cargo.toml")],
            workspace_wide: false,
        });
    }
    let mut files = vec![workspace_root.join("Cargo.toml")];
    for (root, toml) in members {
        if declares_a_number(toml) {
            files.push(root.join("Cargo.toml"));
        }
    }
    Ok(Declaration {
        files,
        workspace_wide: true,
    })
}

/// La racine épingle-t-elle un membre de la famille dans `[workspace.dependencies]` ?
fn pins_the_family(workspace_toml: &str) -> bool {
    let Ok(doc) = workspace_toml.parse::<DocumentMut>() else {
        return false;
    };
    doc.get("workspace")
        .and_then(|workspace| workspace.get("dependencies"))
        .and_then(Item::as_table_like)
        .is_some_and(|deps| SDK_FAMILY.iter().any(|name| deps.contains_key(name)))
}

/// Un membre qui écrit lui-même un numéro pour la famille — pas `workspace = true`.
fn declares_a_number(member_toml: &str) -> bool {
    bump_requirements(member_toml, "0.0.0").is_ok_and(|(_, changed)| !changed.is_empty())
}

/// Rewrites every SDK-family requirement to `target`, keeping the file as it was otherwise.
///
/// Comments, ordering and other keys survive: this is the author's `Cargo.toml`, not a file the
/// CLI owns. An inline table keeps its other keys (`features`, `default-features`…); only its
/// `version` moves. An entry inherited from the workspace is left alone — it has no number here.
/// A root reads `[workspace.dependencies]`, a module its own dependency tables: one pass covers both.
///
/// @returns the new file and the crates it changed, in family order
pub fn bump_requirements(toml: &str, target: &str) -> Result<(String, Vec<String>)> {
    let mut doc: DocumentMut = toml.parse().context("parse Cargo.toml")?;
    let sections: [&[&str]; 4] = [
        &["workspace", "dependencies"],
        &["dependencies"],
        &["dev-dependencies"],
        &["build-dependencies"],
    ];
    let mut changed = Vec::new();
    for path in sections {
        let Some(table) = walk_mut(doc.as_item_mut(), path) else {
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

/// Un contrôle de conformité en échec.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailingCheck {
    pub id: String,
    pub detail: String,
}

/// Ce que la sandbox disait du module avant la montée.
struct Baseline {
    renders: BTreeMap<String, Rendered>,
    /// Les contrôles déjà en échec — la montée n'y est pour rien.
    failing: std::collections::BTreeSet<String>,
}

/// Les échecs de conformité d'après la montée : ceux qu'elle a causés, et ceux d'avant.
///
/// Même règle que pour les rendus ([`RenderOutcome::StillFailing`]) : un contrôle qui échouait
/// déjà n'est pas une régression. Sans elle, un module qui déclare un e-mail sans commande
/// ne pouvait tout simplement plus monter de SDK — la commande restaurait Cargo.toml en
/// accusant la montée d'une faute qu'elle n'avait pas commise.
pub fn split_failures(
    before: &std::collections::BTreeSet<String>,
    after: Vec<FailingCheck>,
) -> (Vec<FailingCheck>, Vec<FailingCheck>) {
    after
        .into_iter()
        .partition(|check| !before.contains(&check.id))
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
    auth_url: String,
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
        let migrations = dev::migrations_bundle(module_root)?;
        let deploying = ui::step(format!("deploying {} to the sandbox", self.module_id));
        let deployed = match dev::deploy(
            &self.base_url,
            &self.module_id,
            &self.token,
            &wasm,
            &manifest,
            migrations.as_deref(),
            self.session.session_id(),
        )
        .await
        {
            Ok(deployed) => deployed,
            // Sur un 401 seulement : renouveler sur n'importe quel échec faisait tourner le
            // jeton pour rien, et chaque rotation de trop est une course de plus avec les autres
            // `portaki` du compte.
            Err(failure) if failure.is::<dev::Unauthorized>() => {
                self.token = dev::renew(&self.auth_url, &self.token).await?;
                dev::deploy(
                    &self.base_url,
                    &self.module_id,
                    &self.token,
                    &wasm,
                    &manifest,
                    migrations.as_deref(),
                    self.session.session_id(),
                )
                .await?
            }
            Err(failure) => {
                deploying.abandon();
                return Err(failure);
            }
        };
        deploying.done(format!("deployed {}", dev::short(&deployed.digest)));

        let client = crate::http::client();
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
    async fn failing_checks(&self) -> Result<Vec<FailingCheck>> {
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
            crate::http::client()
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
            .map(|check| FailingCheck {
                id: check.id,
                detail: check.detail,
            })
            .collect())
    }
}

#[derive(Debug, Deserialize)]
struct Metadata {
    workspace_root: PathBuf,
    /// Où `cargo` écrit, selon la configuration vue depuis le dossier courant.
    target_directory: PathBuf,
    packages: Vec<MetadataPackage>,
    workspace_members: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct MetadataPackage {
    id: String,
    name: String,
    version: String,
    manifest_path: PathBuf,
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
    let cwd = std::env::current_dir().context("current_dir")?;
    let (module_root, from_repository_root) = anchor(&cwd)?;
    let module_id = dev::read_module_id(&module_root)?;
    if from_repository_root {
        ui::field(
            "anchor",
            format!("{module_id} (repository root — every module moves)"),
        );
    }
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
                crate::http::client()
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
    let workspace_toml = std::fs::read_to_string(metadata.workspace_root.join("Cargo.toml")).ok();
    let member_tomls: Vec<(PathBuf, String)> = workspace::members(&metadata.workspace_root)
        .into_iter()
        .filter_map(|member| {
            let toml = std::fs::read_to_string(member.root.join("Cargo.toml")).ok()?;
            Some((member.root, toml))
        })
        .collect();
    let declaration = locate_declaration(
        &module_toml,
        &module_root,
        &metadata.workspace_root,
        workspace_toml.as_deref(),
        &member_tomls,
    )?;
    ui::field(
        "declared in",
        match declaration.files.as_slice() {
            [only] => only.display().to_string(),
            many => format!("{} Cargo.toml (workspace root and modules)", many.len()),
        },
    );
    if declaration.workspace_wide {
        ui::warn(format!(
            "the workspace fixes the SDK for every module: the {} members move together, and all of them are built and tested",
            metadata.workspace_members.len()
        ));
    }

    let members = verified_members(
        &declaration,
        &metadata.workspace_root,
        &module_root,
        &module_id,
    );

    let mut sandbox = if args.no_render {
        ui::skipped("render comparison skipped (--no-render)");
        None
    } else if from_repository_root {
        // Une session de sandbox vise un module : depuis la racine, aucun ne s'impose.
        ui::skipped(
            "render comparison skipped — run from a module directory to compare its renders",
        );
        None
    } else {
        let token = crate::auth::access_token()
            .context("sign in with `portaki login`, or pass --no-render")?;
        let auth_url = crate::auth::api_base_url(args.url.as_deref());
        let session = crate::dev_session::start(&base_url, &auth_url, &module_id, &token).await?;
        Some(Sandbox {
            base_url: base_url.clone(),
            auth_url,
            module_id: module_id.clone(),
            token,
            session,
        })
    };

    let baseline = match sandbox.as_mut() {
        Some(sandbox) => {
            ui::section(&format!("before — portaki-sdk {current}"));
            // Une version actuelle qui ne compile plus est justement une raison de monter :
            // souvent le code attend déjà la suivante. On perd la comparaison des rendus, pas
            // la montée — build, tests et lint la jugent toujours.
            match take_baseline(sandbox, &module_root).await {
                Ok(baseline) => Some(baseline),
                Err(failure) => {
                    ui::warn(format!(
                        "portaki-sdk {current} does not build or render here — upgrading without a render comparison"
                    ));
                    ui::detail(format!("{failure:#}"));
                    None
                }
            }
        }
        None => None,
    };

    ui::section(&format!("after — portaki-sdk {target}"));
    let lock = metadata.workspace_root.join("Cargo.lock");
    let manifests = module_manifests(&metadata, &declaration, &module_root);
    let backup = Backup::take(
        &declaration
            .files
            .iter()
            .cloned()
            .chain([lock])
            .chain(manifests.iter().cloned())
            .collect::<Vec<_>>(),
    );

    let outcome = upgrade_and_verify(
        &args,
        &module_root,
        &declaration,
        &metadata,
        &members,
        &target,
        sandbox.as_mut(),
        baseline.as_ref(),
    )
    .await;
    // La vérification entre dans chaque module : on revient d'où l'on est parti.
    std::env::set_current_dir(&cwd).context("return to the starting directory")?;
    let subject = subject(&module_id, members.len());

    if let Some(sandbox) = &sandbox {
        sandbox.session.release().now().await;
    }
    if args.dry_run {
        backup.restore()?;
    }
    match outcome {
        Ok(resolved) if args.dry_run => {
            ui::blank();
            ui::success(format!(
                "{subject} would move to portaki-sdk {resolved} — dry run, nothing was changed"
            ));
            if baseline.is_some() {
                ui::advice(
                    "the sandbox now holds the upgraded build: run `portaki dev` to put yours back",
                );
            }
            Ok(())
        }
        Ok(resolved) => {
            ui::blank();
            // La version résolue, pas la cible : `"3.0.1"` est un caret, et résout 3.1.0 dès
            // que 3.1.0 existe. Annoncer la cible aurait fait committer un message faux.
            let verb = if members.len() > 1 { "are" } else { "is" };
            ui::success(format!("{subject} {verb} on portaki-sdk {resolved}"));
            // Hérité, le changement est à la racine : un `git diff` lancé depuis le module ne
            // montrerait que son propre manifeste, pas ceux des autres membres.
            let review = if declaration.workspace_wide {
                format!("git -C {} diff", metadata.workspace_root.display())
            } else {
                "git diff Cargo.toml Cargo.lock portaki.module.json".to_string()
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
            if !args.dry_run {
                backup.restore()?;
            }
            ui::blank();
            ui::failure("the upgrade broke something — every file it changed is restored");
            if baseline.is_some() {
                ui::advice("the sandbox now holds the attempted build: run `portaki dev` to put yours back");
            }
            Err(failure)
        }
    }
}

/// Le module d'où partir, et si l'on est à la racine d'un monorepo.
///
/// Depuis la racine, le premier module fait l'affaire : la version y est héritée du workspace,
/// donc c'est la même pour tous, et c'est tout ce que ce module sert à trouver.
fn anchor(cwd: &Path) -> Result<(PathBuf, bool)> {
    if crate::manifest::source::is_module(cwd) {
        return Ok((cwd.to_path_buf(), false));
    }
    let members = workspace::members(cwd);
    match members.first() {
        Some(first) => Ok((first.root.clone(), true)),
        None => anyhow::bail!(
            "no module here, and no modules/*/ below — run from a module or a monorepo root"
        ),
    }
}

/// Les modules à assembler et linter : tous ceux du dépôt quand la version est héritée du
/// workspace, le seul module sinon.
fn verified_members(
    declaration: &Declaration,
    workspace_root: &Path,
    module_root: &Path,
    module_id: &str,
) -> Vec<workspace::Member> {
    let all = if declaration.workspace_wide {
        workspace::members(workspace_root)
    } else {
        Vec::new()
    };
    if all.is_empty() {
        vec![workspace::Member {
            id: module_id.to_string(),
            root: module_root.to_path_buf(),
        }]
    } else {
        all
    }
}

/// « weather » ou « 21 modules » — la phrase dit combien ont bougé.
fn subject(module_id: &str, count: usize) -> String {
    if count > 1 {
        format!("{count} modules")
    } else {
        module_id.to_string()
    }
}

/// Les `portaki.module.json` que la montée concerne : ceux de tous les membres quand la version
/// vient du workspace, celui du module sinon.
fn module_manifests(
    metadata: &Metadata,
    declaration: &Declaration,
    module_root: &Path,
) -> Vec<PathBuf> {
    let roots: Vec<PathBuf> = if declaration.workspace_wide {
        metadata
            .packages
            .iter()
            .filter(|package| metadata.workspace_members.contains(&package.id))
            .filter_map(|package| package.manifest_path.parent().map(Path::to_path_buf))
            .collect()
    } else {
        vec![module_root.to_path_buf()]
    };
    roots
        .into_iter()
        .map(|root| root.join("portaki.module.json"))
        .filter(|path| path.is_file())
        .collect()
}

/// Remplace la valeur de `requiresModuleSdk` sans toucher au reste du fichier — relu et
/// réécrit par serde, un manifeste perdrait son ordre et sa mise en forme dans le diff.
/// `None` quand le champ n'y est pas : le build l'inscrit alors lui-même.
pub fn set_required_sdk(raw: &str, version: &str) -> Option<String> {
    const KEY: &str = "\"requiresModuleSdk\"";
    let after_key = raw.find(KEY)? + KEY.len();
    let colon = after_key + raw[after_key..].find(':')?;
    let open = colon + 1 + raw[colon + 1..].find('"')?;
    let close = open + 1 + raw[open + 1..].find('"')?;
    Some(format!("{}{version}{}", &raw[..=open], &raw[close..]))
}

async fn take_baseline(sandbox: &mut Sandbox, module_root: &Path) -> Result<Baseline> {
    dev::build(module_root)?;
    crate::commands::build::refresh_outputs(module_root)?;
    let renders = sandbox.deploy_and_render(module_root).await?;
    let failing = sandbox.failing_checks().await?;
    for check in &failing {
        ui::detail(format!(
            "conformance {}: failing before the upgrade — {}",
            check.id, check.detail
        ));
    }
    Ok(Baseline {
        renders,
        failing: failing.into_iter().map(|check| check.id).collect(),
    })
}

// Chaque argument est une décision déjà prise par l'appelant (déclaration, membres, sandbox) :
// les regrouper ne ferait que déplacer la liste dans une structure lue à un seul endroit.
#[allow(clippy::too_many_arguments)]
async fn upgrade_and_verify(
    args: &UpgradeArgs,
    module_root: &Path,
    declaration: &Declaration,
    metadata: &Metadata,
    members: &[workspace::Member],
    target: &str,
    sandbox: Option<&mut Sandbox>,
    baseline: Option<&Baseline>,
) -> Result<String> {
    let mut changed: Vec<String> = Vec::new();
    let mut touched = 0;
    for file in &declaration.files {
        let original =
            std::fs::read_to_string(file).with_context(|| format!("read {}", file.display()))?;
        let (bumped, in_file) = bump_requirements(&original, target)?;
        if in_file.is_empty() {
            continue;
        }
        std::fs::write(file, bumped).with_context(|| format!("write {}", file.display()))?;
        touched += 1;
        for name in in_file {
            if !changed.contains(&name) {
                changed.push(name);
            }
        }
    }
    if changed.is_empty() {
        bail!("no SDK requirement with a version number to move");
    }
    ui::wrote(
        "requirement",
        format!("{} → {target} in {touched} Cargo.toml", changed.join(", ")),
    );

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

    // Le build refuse un manifeste qui annonce une autre version que celle liée : sans ceci,
    // toute montée échouait au premier module, avec pour seul conseil d'éditer vingt fichiers.
    let mut aligned = 0;
    for path in &module_manifests(metadata, declaration, module_root) {
        let raw =
            std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        if let Some(updated) = set_required_sdk(&raw, &resolved_version) {
            if updated != raw {
                std::fs::write(path, updated)
                    .with_context(|| format!("write {}", path.display()))?;
                aligned += 1;
            }
        }
    }
    if aligned > 0 {
        ui::wrote(
            "manifests",
            format!("requiresModuleSdk → {resolved_version} in {aligned} portaki.module.json"),
        );
    }

    let scope: &[&str] = if declaration.workspace_wide {
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
    let test: Vec<&str> = ["test"].into_iter().chain(scope.iter().copied()).collect();
    cargo(module_root, "running the tests", &test)?;
    // Sorties et lint pour chaque module qui a bougé, pas seulement celui d'où l'on part : sinon
    // la sortie ne nommait que lui, et l'on croyait qu'il avait monté seul.
    for member in members {
        if members.len() > 1 {
            ui::rule(&member.id);
        }
        // Les émissions sont là où le build du workspace a écrit — pas forcément sous le
        // `target/` du membre, qu'un `.cargo/config.toml` peut fixer ailleurs.
        crate::commands::build::refresh_outputs_from(&member.root, &metadata.target_directory)?;
        workspace::enter(member)?;
        crate::commands::lint::run(crate::commands::lint::LintArgs {
            manifest: None,
            nested: members.len() > 1,
        })?;
    }
    std::env::set_current_dir(module_root).context("return to the module")?;

    let (Some(sandbox), Some(baseline)) = (sandbox, baseline) else {
        return Ok(resolved_version);
    };
    let after = sandbox.deploy_and_render(module_root).await?;

    let mut broken = Vec::new();
    let mut changed_renders = Vec::new();
    for (id, before) in &baseline.renders {
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
    let (introduced, preexisting) =
        split_failures(&baseline.failing, sandbox.failing_checks().await?);
    for check in &preexisting {
        ui::detail(format!(
            "conformance {}: still failing, as before the upgrade",
            check.id
        ));
    }
    for check in introduced {
        broken.push(format!("conformance: {} — {}", check.id, check.detail));
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

    fn monorepo(ids: &[&str]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for id in ids {
            let root = dir.path().join("modules").join(id);
            std::fs::create_dir_all(&root).unwrap();
            std::fs::write(
                root.join("portaki.module.json"),
                format!(r#"{{"id":"{id}","version":"0.1.0"}}"#),
            )
            .unwrap();
        }
        dir
    }

    /// Depuis la racine d'un monorepo, un module sert d'ancre : la version héritée est la même
    /// pour tous.
    #[test]
    fn the_repository_root_anchors_on_a_module() {
        let repo = monorepo(&["weather", "nuki"]);

        let (root, from_repository_root) = anchor(repo.path()).unwrap();

        assert!(from_repository_root);
        assert_eq!(root, repo.path().join("modules/nuki"));
        let (root, from_repository_root) = anchor(&repo.path().join("modules/weather")).unwrap();
        assert!(!from_repository_root);
        assert_eq!(root, repo.path().join("modules/weather"));
        assert!(anchor(tempfile::tempdir().unwrap().path()).is_err());
    }

    /// Hérité du workspace, tous les modules sont vérifiés — pas seulement celui d'où l'on part.
    #[test]
    fn an_inherited_sdk_verifies_every_module() {
        let repo = monorepo(&["weather", "nuki", "wifi-guest"]);
        let weather = repo.path().join("modules/weather");
        let inherited = Declaration {
            files: vec![repo.path().join("Cargo.toml")],
            workspace_wide: true,
        };
        let own = Declaration {
            files: vec![weather.join("Cargo.toml")],
            workspace_wide: false,
        };

        let ids = |members: Vec<workspace::Member>| {
            members
                .into_iter()
                .map(|member| member.id)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            ids(verified_members(
                &inherited,
                repo.path(),
                &weather,
                "weather"
            )),
            vec!["nuki", "weather", "wifi-guest"]
        );
        assert_eq!(
            ids(verified_members(&own, repo.path(), &weather, "weather")),
            vec!["weather"]
        );
        assert_eq!(subject("weather", 1), "weather");
        assert_eq!(subject("weather", 3), "3 modules");
    }

    use super::*;

    #[test]
    fn the_required_sdk_changes_alone() {
        let raw = "{\n  \"id\": \"weather\",\n  \"requiresModuleSdk\": \"6.3.0\",\n  \"version\": \"0.3.24\"\n}\n";
        assert_eq!(
            set_required_sdk(raw, "6.4.0").unwrap(),
            raw.replace("6.3.0", "6.4.0")
        );
        assert_eq!(set_required_sdk("{\"id\": \"weather\"}", "6.4.0"), None);
    }

    #[test]
    fn a_workspace_inherited_sdk_points_at_the_workspace_root() {
        let module = "[package]\nname = \"ical-sync\"\n\n[dependencies]\nportaki-sdk = { workspace = true }\n";
        let declaration =
            locate_declaration(module, Path::new("/m/ical"), Path::new("/m"), None, &[]).unwrap();

        assert_eq!(
            declaration,
            Declaration {
                files: vec![PathBuf::from("/m/Cargo.toml")],
                workspace_wide: true
            }
        );
    }

    #[test]
    fn an_own_requirement_points_at_the_module() {
        let module = "[dependencies]\nportaki-sdk = \"2.4.0\"\n";
        let declaration =
            locate_declaration(module, Path::new("/m/ical"), Path::new("/m"), None, &[]).unwrap();

        assert!(!declaration.workspace_wide);
        assert_eq!(declaration.files, vec![PathBuf::from("/m/ical/Cargo.toml")]);
    }

    /// portaki-modules: the root pins the family and each module writes the same number (so that
    /// release-please sees a bump in every module path). `portaki sdk upgrade` once moved the
    /// anchor module alone, and `check-sdk-pin.sh` failed on main: they move as one.
    #[test]
    fn a_root_pin_copied_into_each_module_moves_everywhere() {
        let root = "[workspace]\nmembers = [\"modules/*\"]\n\n[workspace.dependencies]\nportaki-sdk = \"8.0.0\"\nportaki-sdk-macros = \"8.0.0\"\n";
        let own = |features: &str| {
            format!("[dependencies]\nportaki-sdk = {{ version = \"8.0.0\", features = [{features}] }}\nportaki-sdk-macros = {{ workspace = true }}\n")
        };
        let members = vec![
            (PathBuf::from("/m/modules/access-guide"), own("\"kv\"")),
            (PathBuf::from("/m/modules/nuki"), own("")),
            (
                PathBuf::from("/m/modules/weather"),
                "[dependencies]\nportaki-sdk = { workspace = true }\n".to_string(),
            ),
        ];

        let declaration = locate_declaration(
            &members[0].1,
            &members[0].0,
            Path::new("/m"),
            Some(root),
            &members,
        )
        .unwrap();

        assert!(declaration.workspace_wide);
        assert_eq!(
            declaration.files,
            vec![
                PathBuf::from("/m/Cargo.toml"),
                PathBuf::from("/m/modules/access-guide/Cargo.toml"),
                PathBuf::from("/m/modules/nuki/Cargo.toml"),
            ]
        );
        let (root_after, _) = bump_requirements(root, "8.0.1").unwrap();
        assert!(root_after.contains("portaki-sdk = \"8.0.1\""));
        let (member_after, changed) = bump_requirements(&members[0].1, "8.0.1").unwrap();
        assert!(member_after.contains("portaki-sdk = { version = \"8.0.1\", features = [\"kv\"] }"));
        assert!(member_after.contains("portaki-sdk-macros = { workspace = true }"));
        assert_eq!(changed, vec!["portaki-sdk"]);
    }

    /// Hors workspace, ou sans épinglage à la racine, un module ne fait monter que lui-même.
    #[test]
    fn a_root_without_the_family_leaves_the_module_alone() {
        let root =
            "[workspace]\nmembers = [\"modules/*\"]\n\n[workspace.dependencies]\nserde = \"1\"\n";
        let module = "[dependencies]\nportaki-sdk = \"8.0.0\"\n";

        let declaration = locate_declaration(
            module,
            Path::new("/m/modules/nuki"),
            Path::new("/m"),
            Some(root),
            &[(PathBuf::from("/m/modules/nuki"), module.to_string())],
        )
        .unwrap();

        assert!(!declaration.workspace_wide);
        assert_eq!(
            declaration.files,
            vec![PathBuf::from("/m/modules/nuki/Cargo.toml")]
        );
    }

    /// The workspace root of portaki-modules, as it is.
    #[test]
    fn the_whole_family_moves_and_the_rest_of_the_file_stays() {
        let root = "[workspace]\nmembers = [\"modules/*\"]\n\n[workspace.dependencies]\n# the SDK family moves together\nportaki-sdk = \"2.2.0\"\nportaki-sdk-macros = \"2.2.0\"\nportaki-connectors = \"2.2.0\"\nportaki-test-utils = \"2.2.0\"\nserde = { version = \"1\", features = [\"derive\"] }\n";

        let (bumped, changed) = bump_requirements(root, "3.0.1").unwrap();

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

        let (bumped, changed) = bump_requirements(module, "3.0.1").unwrap();

        assert!(bumped.contains("portaki-sdk = { version = \"3.0.1\", default-features = false }"));
        assert!(bumped.contains("portaki-test-utils = \"3.0.1\""));
        assert_eq!(changed, vec!["portaki-sdk", "portaki-test-utils"]);
    }

    /// No number here: the command must not invent one next to `workspace = true`.
    #[test]
    fn an_inherited_entry_is_left_alone_in_the_module() {
        let module = "[dependencies]\nportaki-sdk = { workspace = true }\n";

        let (bumped, changed) = bump_requirements(module, "3.0.1").unwrap();

        assert!(changed.is_empty());
        assert_eq!(bumped, module);
    }

    #[test]
    fn a_pinned_requirement_stays_pinned() {
        let module = "[dependencies]\nportaki-sdk = \"=2.4.0\"\nportaki-sdk-macros = { version = \"~2.4\" }\n";

        let (bumped, _) = bump_requirements(module, "3.0.1").unwrap();

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

    fn check(id: &str) -> FailingCheck {
        FailingCheck {
            id: id.into(),
            detail: String::new(),
        }
    }

    /// ical-sync declares e-mails without a command: failing before, failing after — not the
    /// upgrade's doing, so it must not roll the upgrade back.
    #[test]
    fn a_check_that_already_failed_is_not_a_regression() {
        let before = ["emails".to_string()].into_iter().collect();

        let (introduced, preexisting) =
            split_failures(&before, vec![check("emails"), check("surfaces")]);

        assert_eq!(introduced, vec![check("surfaces")]);
        assert_eq!(preexisting, vec![check("emails")]);
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
