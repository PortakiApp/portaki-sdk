//! `portaki dev` — build, push to the hosted sandbox, and show what happened.
//!
//! Deliberately **not** a local gateway. A parallel engine always drifts from the real host, so
//! the module runs against the actual runtime in the sandbox; the difference with running
//! locally is latency, not nature — and no line of code leaves the infrastructure.

use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use clap::Parser;
use notify::{RecursiveMode, Watcher};
use sha2::{Digest as _, Sha256};

use crate::ui;

/// How long to wait for the editor to finish writing before rebuilding.
const DEBOUNCE: Duration = Duration::from_millis(300);

/// The module manifest — read on every cycle, and now watched just like the sources.
use crate::manifest::source::MODULE_MANIFEST as MANIFEST;

/// The module's database migrations, replayed on every push.
const MIGRATIONS: &str = "db/migrations";

#[derive(Debug, Parser)]
/// Arguments for `portaki dev`.
pub struct DevArgs {
    /// Rebuild and redeploy on every save; follow the sandbox logs and replay the 7 scenarios
    /// after each deploy.
    #[arg(long)]
    pub watch: bool,

    /// Alias of the global --api, kept for older scripts.
    #[arg(long, hide = true)]
    pub url: Option<String>,

    /// Remove this module from the sandbox, and deploy nothing.
    #[arg(long, conflicts_with_all = ["watch", "dispatch"])]
    pub forget: bool,

    /// Former way to run an operation after each deploy — now `portaki run <operation>`.
    // `num_args = 0..=1`: with no value, `clap` used to refuse with "a value is required" and
    // left you to look the names up elsewhere. That is precisely the moment you don't know them.
    #[arg(long, num_args = 0..=1, default_missing_value = "", hide = true)]
    pub dispatch: Option<String>,

    /// JSON parameters for `--dispatch`.
    #[arg(long, default_value = "{}", hide = true)]
    pub params: String,

    /// `query` reads, `command` writes — the SDK's own distinction.
    #[arg(long, default_value = "query", hide = true)]
    pub kind: String,

    #[command(flatten)]
    pub modules: crate::workspace::ModuleArgs,
}

/// Runs `portaki dev`.
pub async fn run(args: DevArgs) -> Result<()> {
    ui::header(
        "portaki dev",
        &crate::tr!(
            "Runs against the real host in the hosted sandbox — not a local mock.",
            "Tourne contre le vrai hôte, dans la sandbox hébergée — pas une simulation locale."
        ),
    );

    // One module at a time: the sandbox and its lease are held per module.
    crate::workspace::enter(&args.modules.one("dev")?)?;
    let module_root = std::env::current_dir().context("current_dir")?;

    // Old name, kept hidden for two minor releases: it works, and it names the new one.
    if let Some(operation) = args.dispatch.as_deref() {
        ui::warn(crate::tr!(
            "portaki dev --dispatch is now portaki run {} — use that name from now on",
            "portaki dev --dispatch devient portaki run {} — utilisez désormais ce nom",
            if operation.is_empty() {
                "<operation>"
            } else {
                operation
            }
        ));
    }
    // A bare `--dispatch` is not asking for a deployment: it is asking for the names. We show
    // them and stop — compiling and pushing only to end on "which one?" would be a minute lost.
    if args.dispatch.as_deref() == Some("") {
        return list_operations(&module_root);
    }

    let base_url = base_url(&args);
    let mut token = crate::auth::access_token(&base_url)?;
    let module_id = read_module_id(&module_root)?;

    // Before the lease: forgetting is not deploying, and taking the slot only to hand it straight
    // back would make another session wait for nothing.
    if args.forget {
        return forget(&base_url, &module_id, &token).await;
    }

    // Taken for any deployment, `--watch` or not. A one-shot `portaki dev` overwrites the
    // sandbox exactly as a looping session does — once instead of endlessly, which makes it no
    // less surprising for whoever's module has just vanished.
    //
    // Before the first build, not after: refusing once compiled and deployed would already have
    // overwritten what the other session was holding.
    let auth_url = crate::profile::api_url(args.url.as_deref());
    let session = crate::dev_session::start(&base_url, &auth_url, &module_id, &token).await?;

    // Ctrl-c unwinds nothing: without this, the lease would stay taken until it expires and the
    // local lock until the next run. Neither is serious — both recover on their own — but giving
    // the slot back right away avoids a pointless wait.
    {
        // A handle, not the session: a task holding the session would keep its `Drop` from
        // running on the normal return, and the local lock would outlive every failure.
        let release = session.release();
        tokio::spawn(async move {
            if tokio::signal::ctrl_c().await.is_ok() {
                release.now().await;
                ui::blank();
                std::process::exit(130);
            }
        });
    }

    // Opened before the first deployment: what the module logs while starting up counts too.
    // The stream follows the session until ctrl-c; it is also what tells the dock "connected".
    if args.watch {
        tokio::spawn(crate::commands::logs::follow_forever(
            base_url.clone(),
            auth_url.clone(),
            module_id.clone(),
            token.clone(),
        ));
    }

    let mut last_digest = String::new();
    let first = cycle(
        &args,
        &base_url,
        &module_root,
        &module_id,
        &mut token,
        &mut last_digest,
        session.session_id(),
    )
    .await;

    // Handed back as soon as it is no longer needed: a one-off deployment is done, and a failure
    // will hold nothing. The session's `Drop` cannot take care of it — releasing a remote lease
    // means waiting for an answer, which a `Drop` cannot do — so without this a failed build
    // would lock out the next one for a minute and a half.
    if first.is_err() || !args.watch {
        session.release().now().await;
    }
    first?;

    if !args.watch {
        ui::next(&[
            (
                "portaki run <operation>",
                &crate::tr!(
                    "run a query or a command on this build",
                    "lancer une query (lire) ou une command (agir) sur ce build"
                ),
            ),
            (
                "portaki preview <surface>",
                &crate::tr!(
                    "render a surface as hosts and guests will see it",
                    "vérifier le rendu, là où l'hôte et le voyageur le verront"
                ),
            ),
            (
                "portaki check",
                &crate::tr!(
                    "the gate portaki release applies, before publishing",
                    "la porte que portaki release applique, avant de publier"
                ),
            ),
        ]);
        ui::blank();
        return Ok(());
    }

    let src = module_root.join("src");
    let manifest = module_root.join(MANIFEST);
    // Paths are given from the module root, not from the root of the disk: a seventy-character
    // absolute path pushes its explanation onto the next line, and the list stops reading as
    // columns.
    ui::list(
        &crate::tr!("watching", "surveillé"),
        &[
            (
                "src/",
                &crate::tr!(
                    "every save rebuilds and redeploys",
                    "chaque sauvegarde recompile et redéploie"
                ),
            ),
            (
                "Cargo.toml",
                &crate::tr!(
                    "a portaki-sdk feature added or dropped changes the permissions",
                    "une feature de portaki-sdk ajoutée ou retirée change les permissions"
                ),
            ),
            (
                "i18n/",
                &crate::tr!(
                    "the module's name, description and tab labels are read there",
                    "le nom, la description et les libellés d'onglet du module s'y lisent"
                ),
            ),
            (
                "db/migrations/",
                &crate::tr!(
                    "an edited migration replays from zero in the sandbox",
                    "une migration modifiée se rejoue depuis zéro en sandbox"
                ),
            ),
        ],
    );
    ui::blank();
    ui::advice(crate::tr!(
        "a build that fails does not stop the loop — fix and save again",
        "un build en échec n'arrête pas la boucle — corrigez et sauvegardez à nouveau"
    ));
    ui::detail(crate::tr!("from {}", "depuis {}", module_root.display()));

    let (tx, rx) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |event| {
        let _ = tx.send(event);
    })
    .context("start file watcher")?;
    watcher
        .watch(&src, RecursiveMode::Recursive)
        .with_context(|| format!("watch {}", src.display()))?;
    // The manifest too: every cycle re-reads it and sends it, but nothing used to trigger a cycle
    // when it changed. Adding a surface or a permission therefore had no visible effect until the
    // next save of a Rust file — enough to make you believe it is never read.
    //
    // `NonRecursive` on the file itself: watching the module root would pull in `target/`, which
    // every build rewrites — the loop would keep restarting itself for ever.
    //
    // The manifest no longer necessarily exists: what it used to say now comes from the code, from
    // `Cargo.toml` (the features, hence the permissions) and from `i18n/` (names and labels).
    for watched in [manifest, module_root.join("Cargo.toml")] {
        if watched.is_file() {
            watcher
                .watch(&watched, RecursiveMode::NonRecursive)
                .with_context(|| format!("watch {}", watched.display()))?;
        }
    }
    let i18n_dir = module_root.join("i18n");
    if i18n_dir.is_dir() {
        watcher
            .watch(&i18n_dir, RecursiveMode::Recursive)
            .with_context(|| format!("watch {}", i18n_dir.display()))?;
    }
    // An edited migration replays from zero in the sandbox: that still takes a cycle to start.
    let migrations_dir = module_root.join(MIGRATIONS);
    if migrations_dir.is_dir() {
        watcher
            .watch(&migrations_dir, RecursiveMode::Recursive)
            .with_context(|| format!("watch {}", migrations_dir.display()))?;
    }

    loop {
        // Block until the first save…
        if rx.recv().is_err() {
            return Ok(());
        }
        // …then soak up the burst an editor produces while writing a file.
        while rx.recv_timeout(DEBOUNCE).is_ok() {}

        ui::blank();
        ui::rule(&chrono::Local::now().format("%H:%M:%S").to_string());

        if let Err(failure) = cycle(
            &args,
            &base_url,
            &module_root,
            &module_id,
            &mut token,
            &mut last_digest,
            session.session_id(),
        )
        .await
        {
            // A compilation error must not stop the loop: it is the common case.
            ui::report(&failure);
        }
    }
}

/// What this module exposes, and how to call it.
///
/// Read from the manifest, not from the sandbox: the question comes up before the first
/// deployment, and often with no network.
pub(crate) fn list_operations(module_root: &Path) -> Result<()> {
    let (manifest, source) = crate::manifest::load_manifest(module_root, None)?;

    match source {
        crate::manifest::ManifestSource::Built(path) => ui::detail(format!(
            "from {}",
            path.strip_prefix(module_root).unwrap_or(&path).display()
        )),
        crate::manifest::ManifestSource::Emissions => ui::detail(crate::tr!(
            "from the SDK emissions — no build output yet",
            "depuis les émissions du SDK — pas encore de build"
        )),
    }

    if manifest.queries.is_empty() && manifest.commands.is_empty() {
        ui::warn(crate::tr!(
            "{} exposes no operation",
            "{} n'expose aucune opération",
            manifest.id
        ));
        ui::detail(crate::tr!(
            "add a #[portaki_sdk::query] or #[portaki_sdk::command] function, then build",
            "ajoutez une fonction #[portaki_sdk::query] ou #[portaki_sdk::command], puis compilez"
        ));
        ui::blank();
        return Ok(());
    }

    show(
        "queries · read-only",
        manifest
            .queries
            .iter()
            .map(|query| (query.name.as_str(), query.r#fn.as_str())),
    );
    show(
        "commands · mutating",
        manifest
            .commands
            .iter()
            .map(|command| (command.name.as_str(), command.r#fn.as_str())),
    );

    // The example carries a real name from the module: a syntax illustrated on `<operation>`
    // copies across badly, and the `--kind` that goes with it is harder still to guess.
    let sample = manifest
        .queries
        .first()
        .map(|query| query.name.as_str())
        .or_else(|| {
            manifest
                .commands
                .first()
                .map(|command| command.name.as_str())
        })
        .unwrap_or("listThings");

    ui::next(&[(
        &format!("portaki run {sample}"),
        &crate::tr!(
            "run it on the build deployed by portaki dev",
            "la lancer sur le build que portaki dev a déployé"
        ),
    )]);
    ui::blank();
    // `--kind query` is the default: repeating it would teach nothing. `command` is the one you
    // have to remember to pass, and it is precisely the one people forget.
    ui::advice(crate::tr!(
        "--params '{{…}}' passes arguments · the kind is read from the manifest",
        "--params '{{…}}' passe des arguments · le genre se lit dans le manifeste"
    ));
    ui::blank();
    Ok(())
}

/// A group of operations: the name you call, then the function that serves it.
///
/// The Rust symbol is the second column because it is what you go looking for in the sources
/// afterwards; repeating "read-only" on every line would have taught nothing the title does not
/// already say.
fn show<'a>(title: &str, operations: impl Iterator<Item = (&'a str, &'a str)>) {
    let rows: Vec<(&str, &str)> = operations.collect();
    if rows.is_empty() {
        return;
    }
    ui::list(title, &rows);
}

/// One pass: build, upload if it changed, optionally dispatch.
async fn cycle(
    args: &DevArgs,
    base_url: &str,
    module_root: &Path,
    module_id: &str,
    token: &mut String,
    last_digest: &mut String,
    session: Option<&str>,
) -> Result<()> {
    build(module_root)?;
    // What `portaki build` does after compiling, and what `dev` used to skip: regenerating the
    // manifest from the emissions. Without it, the sandbox received the one from the last `build`
    // run by hand — a newly added query stayed invisible until you thought of it.
    crate::commands::build::refresh_outputs(module_root)?;

    // The same resolver as `publish`, and not a guessed path: cargo names the artifact after the
    // target, so `access-guide` produces `access_guide.wasm`.
    let wasm_path = crate::oci::pack::find_wasm_artifact(module_root, module_id)?;
    let wasm = std::fs::read(&wasm_path)
        .with_context(|| format!("read {} — did the build produce it?", wasm_path.display()))?;

    let manifest = sandbox_manifest(module_root)?;
    let migrations = migrations_bundle(module_root)?;

    // The fingerprint covers the Wasm, the manifest AND the migrations. On the Wasm alone, a
    // `--watch` that re-read a modified `portaki.module.json` answered "unchanged" and never sent
    // it: the file was watched for nothing. Same for an edited migration. It stays local — the
    // server-side digest, for its part, is the Wasm's.
    let fingerprint =
        upload_fingerprint(&wasm, &manifest, migrations.as_deref().unwrap_or_default());
    if fingerprint == *last_digest {
        ui::skipped(crate::tr!(
            "unchanged ({}) — nothing to upload",
            "inchangé ({}) — rien à envoyer",
            short(&sha256(&wasm))
        ));
        return Ok(());
    }

    // The result is bound before the match: keeping the call as the match subject would hold the
    // borrow on the token while we are trying to replace it.
    let uploading = ui::step(crate::tr!(
        "deploying {module_id} to the sandbox",
        "déploiement de {module_id} en sandbox"
    ));
    let first = deploy(
        base_url,
        module_id,
        token,
        &wasm,
        &manifest,
        migrations.as_deref(),
        session,
    )
    .await;
    let deployed = match first {
        Err(failure) if failure.is::<Unauthorized>() => {
            uploading.say(crate::tr!(
                "renewing the access token",
                "renouvellement du jeton d'accès"
            ));
            *token = reauthenticate(args, token).await?;
            uploading.say(crate::tr!(
                "deploying {module_id} to the sandbox",
                "déploiement de {module_id} en sandbox"
            ));
            deploy(
                base_url,
                module_id,
                token,
                &wasm,
                &manifest,
                migrations.as_deref(),
                session,
            )
            .await?
        }
        other => other.map_err(|failure| {
            uploading.abandon();
            failure
        })?,
    };
    uploading.done(crate::tr!("deployed {module_id}", "{module_id} déployé"));
    ui::field("digest", short(&deployed.digest));
    ui::field("size", ui::bytes(deployed.size_bytes));
    if let Some(install) = &deployed.install {
        print_install(install);
    }
    *last_digest = fingerprint;

    if args.watch {
        run_scenarios(args, base_url, module_id, token).await;
    }

    if let Some(operation) = &args.dispatch {
        let running = ui::step(format!("dispatching {} {operation}", args.kind));
        let first = dispatch(args, base_url, module_id, token, operation).await;
        let trace = match first {
            Err(failure) if failure.is::<Unauthorized>() => {
                running.say("renewing the access token");
                *token = reauthenticate(args, token).await?;
                dispatch(args, base_url, module_id, token, operation).await?
            }
            other => other.map_err(|failure| {
                running.abandon();
                failure
            })?,
        };
        running.done(format!(
            "{} {operation} — {} ms",
            args.kind, trace.duration_ms
        ));
        print_trace(&trace);
    }
    Ok(())
}

/// One cell of the Scenarios grid: one surface against one pathological case.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub(crate) struct ScenarioCell {
    pub(crate) surface: String,
    pub(crate) case: String,
    /// `ok`, `watch` or `fail`.
    pub(crate) status: String,
    #[serde(default)]
    pub(crate) code: Option<String>,
    #[serde(default)]
    pub(crate) message: Option<String>,
}

/// Replays the seven cases against what has just been deployed, and prints the grid.
///
/// Never fatal: a failing case is what you came here for, not a reason to stop the loop — and a
/// platform that cannot replay them is no reason to stop developing.
async fn run_scenarios(args: &DevArgs, base_url: &str, module_id: &str, token: &mut String) {
    let running = ui::step(crate::tr!(
        "replaying the 7 scenarios",
        "relance des 7 scénarios"
    ));
    let mut outcome = post_scenarios(base_url, module_id, token).await;
    if outcome
        .as_ref()
        .is_err_and(|failure| failure.is::<Unauthorized>())
    {
        if let Ok(renewed) = reauthenticate(args, token).await {
            *token = renewed;
            outcome = post_scenarios(base_url, module_id, token).await;
        }
    }
    let cells = match outcome {
        Ok(cells) => cells,
        Err(failure) => {
            running.abandon();
            ui::warn(crate::tr!(
                "scenarios not replayed — {failure:#}",
                "scénarios non relancés — {failure:#}"
            ));
            return;
        }
    };
    running.done(scenarios_line(&cells));
    print_grid(&cells);
}

/// "scenarios — 12 of 14 ok".
pub(crate) fn scenarios_line(cells: &[ScenarioCell]) -> String {
    let failing = cells.iter().filter(|cell| cell.status != "ok").count();
    crate::tr!(
        "scenarios — {} of {} ok",
        "scénarios — {} sur {} ok",
        cells.len() - failing,
        cells.len()
    )
}

/// The grid, then one line per cell that is not green, with its code and its message.
pub(crate) fn print_grid(cells: &[ScenarioCell]) {
    for row in matrix(cells) {
        ui::detail(row);
    }
    for cell in cells.iter().filter(|cell| cell.status != "ok") {
        let why = [cell.code.as_deref(), cell.message.as_deref()]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" — ");
        let line = format!("{} · {}  {why}", cell.surface, cell.case);
        if cell.status == "fail" {
            ui::failure(line);
        } else {
            ui::warn(line);
        }
    }
}

async fn post_scenarios(base_url: &str, module_id: &str, token: &str) -> Result<Vec<ScenarioCell>> {
    // Patient: the platform renders every surface on every case before answering.
    let response = crate::http::patient_client()
        .post(format!(
            "{base_url}/dev/v1/modules/{module_id}/scenarios/run"
        ))
        .bearer_auth(token)
        .send()
        .await
        .context("replay the scenarios")?;
    read_json(response).await
}

/// The grid, one row per surface and one column per case, in the dock's order.
fn matrix(cells: &[ScenarioCell]) -> Vec<String> {
    let cases = portaki_test_utils::scenarios::CASES;
    // In the order they first turn up, whatever order the answer came in.
    let mut surfaces: Vec<&str> = Vec::new();
    for cell in cells {
        if !surfaces.contains(&cell.surface.as_str()) {
            surfaces.push(&cell.surface);
        }
    }
    let width = surfaces
        .iter()
        .map(|s| s.chars().count())
        .max()
        .unwrap_or(0);
    let mut rows = vec![format!("{:width$}  {}", "", cases.join("  "))];
    for surface in surfaces {
        let marks: Vec<String> = cases
            .iter()
            .map(|case| {
                let mark = match cells
                    .iter()
                    .find(|cell| cell.surface == surface && cell.case == *case)
                    .map(|cell| cell.status.as_str())
                {
                    Some("ok") => "✓",
                    Some("watch") => "!",
                    Some("fail") => "✗",
                    _ => "·",
                };
                format!("{mark:^w$}", w = case.len())
            })
            .collect();
        rows.push(format!("{surface:width$}  {}", marks.join("  ")));
    }
    rows
}

/// An access token lives fifteen minutes; a `--watch` session lives far longer.
///
/// The renewal is attempted once, not in a loop: if the refresh token is out of use too, retrying
/// would only hide the one thing worth saying — you have to log in again.
async fn reauthenticate(args: &DevArgs, stale: &str) -> Result<String> {
    renew(&crate::profile::api_url(args.url.as_deref()), stale).await
}

pub(crate) async fn renew(auth_url: &str, stale: &str) -> Result<String> {
    crate::auth::refresh(auth_url, stale)
        .await
        .context("renew the session — run `portaki login` if this keeps failing")
}

pub(crate) fn build(module_root: &Path) -> Result<()> {
    // Release, not debug: a debug build weighs ten times as much and gets turned away by the 5 MB
    // ingestion cap. Better to compile for longer than to discover the refusal at push time.
    let mut cmd = std::process::Command::new("cargo");
    cmd.current_dir(module_root)
        .args(["build", "--release", "--target", "wasm32-unknown-unknown"]);
    ui::command(
        &crate::tr!(
            "compiling wasm32-unknown-unknown (release)",
            "compilation wasm32-unknown-unknown (release)"
        ),
        &mut cmd,
    )
    .context("cargo build wasm32")
}

/// devapi answers in camelCase, like every Portaki API. Without this rename, `size_bytes` found
/// nothing and the deployment failed while reading its own answer — even though it had succeeded
/// server-side.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeployResponse {
    pub(crate) digest: String,
    pub(crate) size_bytes: u64,
    /// The installation replayed in the sandbox. Absent from a platform that does not replay it.
    #[serde(default)]
    pub(crate) install: Option<InstallCheck>,
}

/// The checklist's `install` cell, as devapi renders it: `PASS`, `FAIL` or `BLOCKED`.
#[derive(Debug, serde::Deserialize)]
pub(crate) struct InstallCheck {
    pub(crate) status: String,
    #[serde(default)]
    pub(crate) detail: String,
}

/// What a host would see when installing: migrations, per-property isolation, reinstallation, two
/// installations at once. Red does not stop the loop — the checklist is what will hold it back.
pub(crate) fn print_install(install: &InstallCheck) {
    let line = format!("installation  {}", install.detail);
    match install.status.as_str() {
        "PASS" => ui::success(line),
        "FAIL" => ui::failure(line),
        _ => ui::warn(line),
    }
}

/// The `migrations.bundle.json` that `refresh_outputs` has just written, if there is one.
///
/// Without it, the sandbox never installed the module's tables: a broken migration was only
/// discovered when a host installed the module.
pub(crate) fn migrations_bundle(module_root: &Path) -> Result<Option<Vec<u8>>> {
    let path = module_root.join("target/portaki/migrations.bundle.json");
    match std::fs::read(&path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(missing) if missing.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(failure) => Err(failure).with_context(|| format!("read {}", path.display())),
    }
}

/// `session` is the lease's session, when we obtained the lease.
///
/// It tells the server that this push is the holder's. Without it — lease not obtained — the
/// server only accepts the push if nobody else holds the slot, which is exactly the guarantee the
/// local lock cannot give.
pub(crate) async fn deploy(
    base_url: &str,
    module_id: &str,
    token: &str,
    wasm: &[u8],
    manifest: &str,
    migrations: Option<&[u8]>,
    session: Option<&str>,
) -> Result<DeployResponse> {
    let mut form = reqwest::multipart::Form::new()
        .part(
            "wasm",
            reqwest::multipart::Part::bytes(wasm.to_vec()).file_name("backend.wasm"),
        )
        .text("manifest", manifest.to_owned());
    if let Some(migrations) = migrations {
        form = form.part(
            "migrations",
            reqwest::multipart::Part::bytes(migrations.to_vec())
                .file_name("migrations.bundle.json"),
        );
    }
    if let Some(session) = session {
        form = form.text("sessionId", session.to_owned());
    }

    // Patient: a `.wasm` of several megabytes goes out from here, and cutting it off after fifteen
    // seconds would break an ordinary deployment. The connect timeout, for its part, stays short.
    let response = crate::http::patient_client()
        .post(format!(
            "{}/dev/v1/modules/{module_id}/dev-deploy",
            base_url
        ))
        .bearer_auth(token)
        .multipart(form)
        .send()
        .await
        .context("upload to the dev platform")?;

    read_json(response).await
}

/// Removes this module from the sandbox.
///
/// A dev deployment is overwritten on every push but never deleted: a module tried once stayed in
/// the inventory, next to the ones actually being worked on. Nothing is recompiled here — it is a
/// row being removed, not an artifact being replaced.
async fn forget(base_url: &str, module_id: &str, token: &str) -> Result<()> {
    let forgetting = ui::step(crate::tr!(
        "forgetting {module_id}",
        "retrait de {module_id}"
    ));
    let response = crate::http::client()
        .delete(format!("{base_url}/dev/v1/modules/{module_id}/dev-deploy"))
        .bearer_auth(token)
        .send()
        .await
        .map_err(|failure| {
            forgetting.abandon();
            failure
        })
        .context("ask the dev platform to forget this module")?;

    let status = response.status();
    if !status.is_success() {
        forgetting.abandon();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("the dev platform answered {status}: {}", body.trim());
    }
    forgetting.done(crate::tr!(
        "{module_id} is gone from the sandbox",
        "{module_id} a quitté la sandbox"
    ));
    ui::advice(crate::tr!(
        "its inventory row is what goes — a published version, if any, is untouched",
        "c'est sa ligne d'inventaire qui part — une version publiée, s'il y en a, reste intacte"
    ));
    ui::blank();
    Ok(())
}

/// Same remark, only worse: the `serde(default)`s below swallowed the mismatch in silence.
/// `--dispatch` printed an empty result, a zero duration and no host call at all for an invocation
/// that had run perfectly.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DispatchResponse {
    #[serde(default)]
    result_json: String,
    #[serde(default)]
    pub(crate) duration_ms: u64,
    /// What the runtime refused; absent when the call went through.
    #[serde(default)]
    pub(crate) error_code: Option<String>,
    #[serde(default)]
    host_calls: Vec<HostCall>,
    #[serde(default)]
    captured_effects: Vec<CapturedEffect>,
    #[serde(default)]
    published_events: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct HostCall {
    op: String,
    duration_micros: u64,
    #[serde(default)]
    error_code: String,
    /// What the module asked for. The sandbox keeps the values, production does not: absent means
    /// "the runtime did not pass them on", not "the module passed nothing".
    #[serde(default)]
    args_json: Option<String>,
    #[serde(default)]
    result_json: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct CapturedEffect {
    op: String,
    detail_json: String,
}

async fn dispatch(
    args: &DevArgs,
    base_url: &str,
    module_id: &str,
    token: &str,
    operation: &str,
) -> Result<DispatchResponse> {
    let body = serde_json::json!({
        "operation": operation,
        "kind": args.kind,
        "paramsJson": args.params,
    });
    // Patient too: the platform runs the operation before answering.
    let response = crate::http::patient_client()
        .post(format!("{}/dev/v1/modules/{module_id}/dispatch", base_url))
        .bearer_auth(token)
        .json(&body)
        .send()
        .await
        .context("dispatch on the dev platform")?;

    read_json(response).await
}

/// Prints what the run did — and what the sandbox refused to do.
pub(crate) fn print_trace(trace: &DispatchResponse) {
    if !trace.host_calls.is_empty() || !trace.captured_effects.is_empty() {
        ui::detail(crate::tr!(
            "what the run asked the host for:",
            "ce que l'exécution a demandé à l'hôte :"
        ));
    }
    for call in &trace.host_calls {
        // In call order, the module's own voice in among what it asked for: a `log` lifted out of
        // the list would say everything except which calls it was written between.
        if let Some(line) = log_line(call) {
            ui::detail(line);
            continue;
        }
        let outcome = if call.error_code.is_empty() {
            String::new()
        } else {
            format!("  ← {}", call.error_code)
        };
        ui::detail(format!(
            "{:>7} µs  {}{}",
            call.duration_micros, call.op, outcome
        ));
        if ui::verbose() {
            if let Some(args) = value_line("args", call.args_json.as_deref()) {
                ui::detail(args);
            }
            if let Some(result) = value_line("result", call.result_json.as_deref()) {
                ui::detail(result);
            }
        }
    }
    for effect in &trace.captured_effects {
        ui::detail(format!("captured  {}  {}", effect.op, effect.detail_json));
    }
    for event in &trace.published_events {
        ui::detail(format!("would publish  {event}"));
    }
    // "captured" and "would publish" look alike enough to be taken for things actually done.
    // They are not: the sandbox records them and holds them back.
    if !trace.captured_effects.is_empty() || !trace.published_events.is_empty() {
        ui::detail(crate::tr!(
            "captured and would-publish lines were held, not performed",
            "les lignes captured et would-publish ont été retenues, pas exécutées"
        ));
    }
    if !trace.result_json.is_empty() {
        ui::result(&trace.result_json);
    }
}

/// Beyond that, a value drowns the trace — the whole detail can be read in the developer space.
const VALUE_WIDTH: usize = 160;

/// A line written by the module itself, or `None` if this call is not a `host::log`.
///
/// A `log` goes through the same channel as `kv.get`: without reading its arguments, the trace
/// printed "41 µs  log", and the message the developer had just written stayed invisible.
fn log_line(call: &HostCall) -> Option<String> {
    if call.op != "log" {
        return None;
    }
    let args: serde_json::Value = serde_json::from_str(call.args_json.as_deref()?).ok()?;
    let level = args
        .get("level")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("info");
    let message = args.get("message").and_then(serde_json::Value::as_str)?;
    let fields = args
        .get("fieldsJson")
        .and_then(serde_json::Value::as_str)
        .filter(|fields| !fields.is_empty() && *fields != "{}")
        .map(|fields| format!("  {}", truncate(fields)))
        .unwrap_or_default();
    // Five spaces, not two: the message lands under the operations column, where "µs" shifts the
    // other lines across.
    Some(format!("{level:>7}     {message}{fields}"))
}

/// What a call asked for or received, under its line. Nothing when the runtime did not pass it on.
fn value_line(label: &str, value: Option<&str>) -> Option<String> {
    let value = value?.trim();
    if value.is_empty() {
        return None;
    }
    Some(format!("{label:>7}  {}", truncate(value)))
}

fn truncate(value: &str) -> String {
    if value.chars().count() <= VALUE_WIDTH {
        return value.to_string();
    }
    let kept: String = value.chars().take(VALUE_WIDTH).collect();
    format!("{kept}…")
}

/// The one failure we know what to do with: renew and replay. It carries a type so the caller can
/// tell it apart from a 500, which it would be absurd to replay with a different token.
#[derive(Debug)]
pub(crate) struct Unauthorized;

impl std::fmt::Display for Unauthorized {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the dev platform refused the token")
    }
}

impl std::error::Error for Unauthorized {}

pub(crate) async fn read_json<T: serde::de::DeserializeOwned>(
    response: reqwest::Response,
) -> Result<T> {
    let status = response.status();
    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Err(anyhow::Error::new(Unauthorized));
    }
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        bail!("the dev platform answered {status}: {body}");
    }
    serde_json::from_str(&body).with_context(|| format!("unexpected answer: {body}"))
}

/// The CLI's single base URL — `--url` is now no more than a hidden alias of `--api`.
fn base_url(args: &DevArgs) -> String {
    crate::profile::api_url(args.url.as_deref())
}

pub(crate) fn read_module_id(module_root: &Path) -> Result<String> {
    crate::manifest::source::module_id(module_root).with_context(|| {
        format!(
            "{} is not a module — run from the module root",
            module_root.display()
        )
    })
}

/// The manifest the sandbox receives: the hand-written one if it exists, the crate's
/// `{ id, version }` otherwise, stamped with the SDK version cargo resolved and with what the
/// build emitted.
///
/// Shared with `portaki sdk upgrade`, which deploys twice — before and after the version bump —
/// and must send exactly what `dev` would send.
pub(crate) fn sandbox_manifest(module_root: &Path) -> Result<String> {
    let raw_manifest = crate::manifest::source::source_manifest(module_root)?;
    // What the code says about the module — name, icon, maturity… — fills in what the manifest
    // leaves unsaid.
    let raw_manifest =
        match std::fs::read_to_string(module_root.join(crate::manifest::catalog::BUILT_CATALOG)) {
            Ok(catalog) => crate::manifest::catalog::fill_catalog(&raw_manifest, &catalog)?,
            Err(_) => raw_manifest,
        };
    // The same stamp as `publish`, and for the same reason: `requiresModuleSdk` names the set of
    // contracts an SDUI tree is to be typed against, and it can only be right if it comes from the
    // graph cargo resolved. Without it, the sandbox received a mute manifest and the inspector
    // refused to type — for every module, always.
    let manifest = crate::oci::pack::stamp_sdk_version(
        &raw_manifest,
        crate::oci::pack::resolved_sdk_version(module_root)?,
    )?;
    // And what the build emitted — surfaces, queries, commands. Without the surfaces, the sandbox
    // takes the `pathSegment` for an identifier and asks for a symbol that does not exist; without
    // the operations, all it can offer is free-text entry of the name to dispatch.
    let manifest =
        match std::fs::read_to_string(module_root.join(crate::manifest::loader::BUILT_MANIFEST)) {
            Ok(built) => crate::oci::pack::stamp_built_declarations(&manifest, &built)?,
            // No build manifest: we send what we have, as before.
            Err(_) => manifest,
        };
    Ok(manifest)
}

/// What decides whether a cycle has something to send: the binary, the manifest and the
/// migrations together.
fn upload_fingerprint(wasm: &[u8], manifest: &str, migrations: &[u8]) -> String {
    sha256(&[wasm, b"\0", manifest.as_bytes(), b"\0", migrations].concat())
}

fn sha256(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

/// Digests are unreadable in full; the first bytes are enough to tell two builds apart.
pub(crate) fn short(digest: &str) -> String {
    digest.chars().take("sha256:".len() + 12).collect()
}

#[cfg(test)]
mod tests {
    use super::DevArgs;
    use clap::Parser;

    #[test]
    fn the_scenario_matrix_reads_like_the_dock() {
        let cell = |surface: &str, case: &str, status: &str| ScenarioCell {
            surface: surface.into(),
            case: case.into(),
            status: status.into(),
            code: None,
            message: None,
        };
        let cells = [
            cell("home.card", "normal", "ok"),
            cell("home.card", "no_email", "watch"),
            cell("home.card", "no_photo", "fail"),
        ];

        let rows = matrix(&cells);

        assert_eq!(rows.len(), 2);
        assert!(
            rows[0].trim_start().starts_with("normal  no_email"),
            "{}",
            rows[0]
        );
        assert!(
            rows[1].starts_with("home.card    ✓        !    "),
            "{}",
            rows[1]
        );
        assert!(rows[1].trim_end().ends_with('✗'), "{}", rows[1]);
    }

    /// Forgetting is not deploying: the two flags that push are refused alongside it.
    #[test]
    fn forget_does_not_go_with_the_flags_that_deploy() {
        let forgetting = DevArgs::try_parse_from(["dev", "--forget"]).expect("forget alone");
        assert!(forgetting.forget);
        assert!(!forgetting.watch && forgetting.dispatch.is_none());

        for pushing in [
            vec!["dev", "--forget", "--watch"],
            vec!["dev", "--forget", "--dispatch", "getConfig"],
        ] {
            assert!(DevArgs::try_parse_from(&pushing).is_err(), "{pushing:?}");
        }
    }

    /// A manifest changed without touching the code must go out again: that is what `--watch`
    /// is watching for.
    #[test]
    fn a_manifest_change_alone_is_something_to_upload() {
        let wasm = b"\0asm same bytes";
        assert_ne!(
            upload_fingerprint(wasm, r#"{"version":"0.4.0"}"#, b""),
            upload_fingerprint(wasm, r#"{"version":"0.4.1"}"#, b"")
        );
        assert_eq!(
            upload_fingerprint(wasm, "{}", b""),
            upload_fingerprint(wasm, "{}", b"")
        );
    }

    /// An edited migration, without touching the code or the manifest, must go out again too.
    #[test]
    fn a_migration_change_alone_is_something_to_upload() {
        let wasm = b"\0asm same bytes";
        assert_ne!(
            upload_fingerprint(
                wasm,
                "{}",
                br#"{"revisions":[{"sql":"CREATE TABLE a (id INT);"}]}"#
            ),
            upload_fingerprint(
                wasm,
                "{}",
                br#"{"revisions":[{"sql":"CREATE TABLE b (id INT);"}]}"#
            )
        );
    }

    #[test]
    fn the_migrations_bundle_is_the_one_the_build_wrote() {
        let dir = tempfile::tempdir().unwrap();
        assert!(migrations_bundle(dir.path()).unwrap().is_none());

        std::fs::create_dir_all(dir.path().join("target/portaki")).unwrap();
        std::fs::write(
            dir.path().join("target/portaki/migrations.bundle.json"),
            "{}",
        )
        .unwrap();

        assert_eq!(
            migrations_bundle(dir.path()).unwrap().as_deref(),
            Some(&b"{}"[..])
        );
    }

    /// A platform that does not replay the installation says nothing about it: the answer stays
    /// readable.
    #[test]
    fn a_deploy_response_reads_the_install_outcome_when_there_is_one() {
        let body = r#"{"digest":"sha256:ab","sizeBytes":1,
            "install":{"id":"install","status":"FAIL",
              "detail":"installation : migration_apply_failed revision=002_add_status"}}"#;

        let parsed: DeployResponse = serde_json::from_str(body).expect("réponse lisible");

        let install = parsed.install.expect("install rendu");
        assert_eq!(install.status, "FAIL");
        assert!(install.detail.contains("002_add_status"));
    }

    use super::*;

    /// Value obtained with `printf '\0asm' | shasum -a 256`, not copied back from the test output.
    #[test]
    fn a_digest_is_computed_on_the_bytes() {
        assert_eq!(
            sha256(b"\0asm"),
            "sha256:cd5d4935a48c0672cb06407bb443bc0087aff947c6b864bac886982c73b3027f"
        );
    }

    #[test]
    fn a_short_digest_stays_recognisable() {
        assert_eq!(short("sha256:abcdef0123456789"), "sha256:abcdef012345");
    }

    #[test]
    fn the_module_id_comes_from_the_manifest() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("portaki.module.json"),
            r#"{"id":"nuki","version":"1.4.0"}"#,
        )
        .unwrap();

        assert_eq!(read_module_id(dir.path()).unwrap(), "nuki");
    }

    /// The exact payload devapi returns, copied from a real deployment.
    ///
    /// This is the test that was missing: the struct expected `size_bytes`, the answer carried
    /// `sizeBytes`, and the deployment failed to read its own success.
    #[test]
    fn a_deploy_response_is_read_as_devapi_writes_it() {
        let body = r#"{"moduleId":"access-guide","version":"0.3.2",
            "digest":"sha256:3e9fc0c17866a50a005799fc53c854e696a101a9a6be0884b701cf157b4a9d1a",
            "permissions":["email","kv","platform"],"sizeBytes":1024374,
            "deployedAt":"2026-09-09T10:21:22.119744997Z"}"#;

        let parsed: DeployResponse = serde_json::from_str(body).expect("réponse de deploy lisible");

        assert_eq!(parsed.size_bytes, 1_024_374);
        assert!(parsed.digest.starts_with("sha256:"));
        assert!(
            parsed.install.is_none(),
            "devapi antérieur : pas d'installation rejouée"
        );
    }

    /// The `serde(default)`s of a dispatch answer swallow a mismatch in silence: without
    /// assertions on the values, a deserialisation test would pass on nothing at all.
    #[test]
    fn a_dispatch_response_carries_its_values_not_defaults() {
        let body = r#"{"runId":"4d7a","hasResult":true,"resultJson":"{\"ok\":true}",
            "durationMs":42,"publishedEvents":[],
            "capturedEffects":[{"op":"email.send","detailJson":"{}","at":"2026-09-09T10:00:00Z"}],
            "hostCalls":[{"op":"kv.get","durationMicros":128,"errorCode":""}]}"#;

        let parsed: DispatchResponse =
            serde_json::from_str(body).expect("réponse de dispatch lisible");

        assert_eq!(parsed.duration_ms, 42);
        assert_eq!(parsed.host_calls.len(), 1);
        assert_eq!(parsed.host_calls[0].duration_micros, 128);
        assert_eq!(parsed.captured_effects[0].detail_json, "{}");
    }

    /// The sandbox passes the values on; the CLI used to ignore them, and that is the channel the
    /// module's log lines travel through.
    #[test]
    fn a_host_call_keeps_the_values_the_sandbox_sent() {
        let body = r#"{"runId":"4d7a","hasResult":false,"resultJson":"","durationMs":3,
            "hostCalls":[{"op":"kv.get","durationMicros":128,"errorCode":"",
            "argsJson":"{\"key\":\"wifi\"}","resultJson":"{\"value\":\"soleil\"}"}]}"#;

        let parsed: DispatchResponse =
            serde_json::from_str(body).expect("réponse de dispatch lisible");

        assert_eq!(
            parsed.host_calls[0].args_json.as_deref(),
            Some(r#"{"key":"wifi"}"#)
        );
        assert_eq!(
            parsed.host_calls[0].result_json.as_deref(),
            Some(r#"{"value":"soleil"}"#)
        );
    }

    /// "41 µs  log" did not say what the module had written — which is the whole point of a log.
    #[test]
    fn a_log_call_reads_as_the_line_the_module_wrote() {
        let call = HostCall {
            op: "log".into(),
            duration_micros: 41,
            error_code: String::new(),
            args_json: Some(
                r#"{"level":"warn","message":"clé absente","fieldsJson":"{\"key\":\"wifi\"}"}"#
                    .into(),
            ),
            result_json: None,
        };

        let line = log_line(&call).expect("une ligne de journal");

        assert!(line.contains("warn"), "{line}");
        assert!(line.contains("clé absente"), "{line}");
        assert!(line.contains(r#"{"key":"wifi"}"#), "{line}");
    }

    /// Empty fields would add "{}" at the end of every line, for nothing.
    #[test]
    fn a_log_line_drops_empty_fields() {
        let call = HostCall {
            op: "log".into(),
            duration_micros: 12,
            error_code: String::new(),
            args_json: Some(r#"{"level":"info","message":"prêt","fieldsJson":"{}"}"#.into()),
            result_json: None,
        };

        assert_eq!(log_line(&call).unwrap().trim_end(), "   info     prêt");
    }

    /// Outside the sandbox, the runtime does not pass the values on: the log line is then
    /// unavailable, and the call must still be shown as an ordinary host call.
    #[test]
    fn a_call_without_values_is_not_a_log_line() {
        let without_values = HostCall {
            op: "log".into(),
            duration_micros: 41,
            error_code: String::new(),
            args_json: None,
            result_json: None,
        };
        let other_op = HostCall {
            op: "kv.get".into(),
            duration_micros: 41,
            error_code: String::new(),
            args_json: Some(r#"{"key":"wifi"}"#.into()),
            result_json: None,
        };

        assert!(log_line(&without_values).is_none());
        assert!(log_line(&other_op).is_none());
    }

    #[test]
    fn a_value_line_is_cut_before_it_floods_the_trace() {
        let long = format!("{{\"v\":\"{}\"}}", "a".repeat(400));

        let line = value_line("result", Some(&long)).expect("une ligne de valeur");

        assert!(line.ends_with('…'), "{line}");
        assert!(line.chars().count() <= VALUE_WIDTH + 12, "{line}");
        assert!(value_line("result", None).is_none());
        assert!(value_line("result", Some("   ")).is_none());
    }
}
