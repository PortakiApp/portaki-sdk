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

/// Le manifeste du module — lu à chaque cycle, et désormais surveillé comme les sources.
const MANIFEST: &str = "portaki.module.json";

#[derive(Debug, Parser)]
/// Arguments for `portaki dev`.
pub struct DevArgs {
    /// Rebuild and redeploy on every save.
    #[arg(long)]
    pub watch: bool,

    /// Base URL of the dev platform. Defaults to PORTAKI_DEV_URL, then PORTAKI_API_URL,
    /// then production.
    #[arg(long)]
    pub url: Option<String>,

    /// Operation to dispatch after each deploy. Bare, it lists what this module exposes.
    ///
    /// `num_args = 0..=1` : sans valeur, `clap` refusait avec « a value is required » et
    /// laissait chercher les noms ailleurs. C'est pourtant le moment où on ne les connaît pas.
    #[arg(long, num_args = 0..=1, default_missing_value = "")]
    pub dispatch: Option<String>,

    /// JSON parameters for `--dispatch`.
    #[arg(long, default_value = "{}")]
    pub params: String,

    /// `query` reads, `command` writes — the SDK's own distinction.
    #[arg(long, default_value = "query")]
    pub kind: String,
}

/// Runs `portaki dev`.
pub async fn run(args: DevArgs) -> Result<()> {
    ui::header(
        "portaki dev",
        "Runs against the real host in the hosted sandbox — not a local mock.",
    );

    let module_root = std::env::current_dir().context("current_dir")?;

    // `--dispatch` nu ne demande pas un déploiement : il demande les noms. On les montre et on
    // s'arrête — compiler et pousser pour finir sur « laquelle ? » serait une minute perdue.
    if args.dispatch.as_deref() == Some("") {
        return list_operations(&module_root);
    }

    let mut token = crate::auth::access_token()?;
    let module_id = read_module_id(&module_root)?;
    let base_url = base_url(&args);

    // Prise pour tout déploiement, `--watch` ou non. Un `portaki dev` seul écrase le bac à
    // sable exactement comme une session qui boucle — une fois au lieu de sans fin, ce qui ne
    // le rend pas moins surprenant pour celui dont le module vient de disparaître.
    //
    // Avant le premier build, pas après : refuser une fois compilé et déployé aurait déjà
    // écrasé ce que l'autre session tenait.
    let session = crate::dev_session::start(&base_url, &module_id, &token).await?;

    // Ctrl-c ne déroule rien : sans ceci, le bail resterait pris jusqu'à son échéance et le
    // verrou local jusqu'au prochain lancement. Ni l'un ni l'autre n'est grave — les deux se
    // reprennent seuls — mais rendre la place tout de suite évite une attente pour rien.
    {
        // Une poignée, pas la session : une tâche qui la retiendrait empêcherait son `Drop` de
        // s'exécuter au retour normal, et le verrou local survivrait à chaque échec.
        let release = session.release();
        tokio::spawn(async move {
            if tokio::signal::ctrl_c().await.is_ok() {
                release.now().await;
                ui::blank();
                std::process::exit(130);
            }
        });
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

    // Rendue dès qu'on n'en a plus besoin : un déploiement ponctuel a fini, et un échec ne
    // gardera rien. Le `Drop` de la session ne peut pas s'en charger — rendre un bail distant
    // demande d'attendre une réponse, ce qu'un `Drop` ne sait pas faire — donc sans ceci un
    // build raté interdirait le suivant pendant une minute et demie.
    if first.is_err() || !args.watch {
        session.release().now().await;
    }
    first?;

    if !args.watch {
        ui::blank();
        return Ok(());
    }

    let src = module_root.join("src");
    let manifest = module_root.join(MANIFEST);
    // Les chemins sont dits depuis la racine du module, pas depuis celle du disque : un chemin
    // absolu de soixante-dix caractères repousse son explication à la ligne suivante, et la
    // liste cesse de se lire en colonnes.
    ui::list(
        "watching",
        &[
            (
                "src/",
                "every save rebuilds, redeploys and dispatches again",
            ),
            (
                MANIFEST,
                "surfaces and permissions take effect without touching a .rs file",
            ),
        ],
    );
    ui::blank();
    ui::advice("a build that fails does not stop the loop — fix and save again");
    ui::detail(format!("from {}", module_root.display()));

    let (tx, rx) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |event| {
        let _ = tx.send(event);
    })
    .context("start file watcher")?;
    watcher
        .watch(&src, RecursiveMode::Recursive)
        .with_context(|| format!("watch {}", src.display()))?;
    // Le manifeste aussi : chaque cycle le relit et l'envoie, mais rien ne déclenchait de cycle
    // quand il changeait. Ajouter une surface ou une permission restait donc sans effet visible
    // jusqu'à la prochaine sauvegarde d'un fichier Rust — de quoi croire qu'il n'est pas lu.
    //
    // `NonRecursive` sur le fichier lui-même : surveiller la racine du module ferait entrer
    // `target/`, que chaque build réécrit — la boucle se relancerait elle-même sans fin.
    watcher
        .watch(&manifest, RecursiveMode::NonRecursive)
        .with_context(|| format!("watch {}", manifest.display()))?;

    loop {
        // Bloque jusqu'à la première sauvegarde…
        if rx.recv().is_err() {
            return Ok(());
        }
        // …puis absorbe la rafale qu'un éditeur produit en écrivant un fichier.
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
            // Une erreur de compilation ne doit pas arrêter la boucle : c'est le cas courant.
            ui::report(&failure);
        }
    }
}

/// Ce que ce module expose, et comment l'appeler.
///
/// Lu du manifeste, pas de la sandbox : la question se pose avant le premier déploiement, et
/// souvent sans réseau.
fn list_operations(module_root: &Path) -> Result<()> {
    let (manifest, source) = crate::manifest::load_manifest(module_root, None)?;

    match source {
        crate::manifest::ManifestSource::Built(path) => ui::detail(format!(
            "from {}",
            path.strip_prefix(module_root).unwrap_or(&path).display()
        )),
        crate::manifest::ManifestSource::Emissions => {
            ui::detail("from the SDK emissions — no build output yet")
        }
    }

    if manifest.queries.is_empty() && manifest.commands.is_empty() {
        ui::warn(format!("{} exposes no operation", manifest.id));
        ui::detail("add a #[portaki_sdk::query] or #[portaki_sdk::command] function, then build");
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

    // L'exemple porte un vrai nom du module : une syntaxe illustrée sur `<operation>` se recopie
    // mal, et le `--kind` qui va avec se devine encore moins.
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
        &format!("portaki dev --dispatch {sample}"),
        "build, deploy, then run it",
    )]);
    ui::blank();
    // `--kind query` est le défaut : le rappeler n'apprendrait rien. C'est `command` qu'il faut
    // penser à poser, et c'est justement celui qu'on oublie.
    ui::advice("--kind command for a mutating one · --params '{…}' passes arguments");
    ui::blank();
    Ok(())
}

/// Un groupe d'opérations : le nom qu'on appelle, puis la fonction qui le sert.
///
/// Le symbole Rust est la seconde colonne parce que c'est lui qu'on cherche ensuite dans les
/// sources ; répéter « read-only » à chaque ligne n'aurait rien appris que le titre ne dise.
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
    // Ce que `portaki build` fait après la compilation, et que `dev` sautait : régénérer le
    // manifeste depuis les émissions. Sans ça, la sandbox recevait celui du dernier `build`
    // lancé à la main — une requête ajoutée restait invisible jusqu'à ce qu'on y pense.
    crate::commands::build::refresh_outputs(module_root)?;

    // Le même résolveur que `publish`, et pas un chemin deviné : cargo nomme l'artefact
    // d'après la cible, donc `access-guide` produit `access_guide.wasm`.
    let wasm_path = crate::oci::pack::find_wasm_artifact(module_root, module_id)?;
    let wasm = std::fs::read(&wasm_path)
        .with_context(|| format!("read {} — did the build produce it?", wasm_path.display()))?;

    let raw_manifest =
        std::fs::read_to_string(module_root.join(MANIFEST)).context("read portaki.module.json")?;
    // Le même tampon que `publish`, et pour la même raison : `requiresModuleSdk` désigne le jeu
    // de contrats contre lequel typer un arbre SDUI, et il ne peut être exact que s'il vient du
    // graphe résolu par cargo. Sans lui, la sandbox recevait un manifeste muet et
    // l'inspecteur refusait de typer — pour tous les modules, toujours.
    let manifest = crate::oci::pack::stamp_sdk_version(
        &raw_manifest,
        crate::oci::pack::resolved_sdk_version(module_root)?,
    )?;
    // Et ce que le build a emis — surfaces, queries, commands. Sans les surfaces, la sandbox
    // prend le `pathSegment` pour un identifiant et demande un symbole qui n'existe pas ; sans
    // les operations, elle ne peut proposer qu'une saisie libre du nom a dispatcher.
    let manifest =
        match std::fs::read_to_string(module_root.join(crate::manifest::loader::BUILT_MANIFEST)) {
            Ok(built) => crate::oci::pack::stamp_built_declarations(&manifest, &built)?,
            // Pas de manifeste de build : on envoie ce qu'on a, comme avant.
            Err(_) => manifest,
        };

    // L'empreinte porte sur le Wasm ET le manifeste. Sur le seul Wasm, un `--watch` qui relisait
    // `portaki.module.json` modifié répondait « unchanged » et ne l'envoyait jamais : le fichier
    // était surveillé pour rien. Elle reste locale — le digest serveur, lui, est celui du Wasm.
    let fingerprint = upload_fingerprint(&wasm, &manifest);
    if fingerprint == *last_digest {
        ui::skipped(format!(
            "unchanged ({}) — nothing to upload",
            short(&sha256(&wasm))
        ));
        return Ok(());
    }

    // Le résultat est lié avant le match : garder l'appel comme sujet du match retiendrait
    // l'emprunt du jeton pendant qu'on cherche à le remplacer.
    let uploading = ui::step(format!("deploying {module_id} to the sandbox"));
    let first = deploy(base_url, module_id, token, &wasm, &manifest, session).await;
    let deployed = match first {
        Err(failure) if failure.is::<Unauthorized>() => {
            uploading.say("renewing the access token");
            *token = reauthenticate().await?;
            uploading.say(format!("deploying {module_id} to the sandbox"));
            deploy(base_url, module_id, token, &wasm, &manifest, session).await?
        }
        other => other.map_err(|failure| {
            uploading.abandon();
            failure
        })?,
    };
    uploading.done(format!("deployed {module_id}"));
    ui::field("digest", short(&deployed.digest));
    ui::field("size", ui::bytes(deployed.size_bytes));
    *last_digest = fingerprint;

    if let Some(operation) = &args.dispatch {
        let running = ui::step(format!("dispatching {} {operation}", args.kind));
        let first = dispatch(args, base_url, module_id, token, operation).await;
        let trace = match first {
            Err(failure) if failure.is::<Unauthorized>() => {
                running.say("renewing the access token");
                *token = reauthenticate().await?;
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

/// Un jeton d'accès vit quinze minutes ; une session `--watch` bien plus longtemps.
///
/// Le renouvellement est tenté une fois, pas en boucle : si le jeton de rafraîchissement est
/// lui aussi hors d'usage, réessayer ne ferait que masquer la seule chose à dire — il faut se
/// reconnecter.
async fn reauthenticate() -> Result<String> {
    crate::auth::refresh()
        .await
        .context("renew the session — run `portaki login` if this keeps failing")
}

fn build(module_root: &Path) -> Result<()> {
    // Release, pas debug : un build debug pèse dix fois plus et se fait refuser par le plafond
    // d'ingestion de 5 Mo. Mieux vaut compiler plus longtemps que découvrir le refus au push.
    let mut cmd = std::process::Command::new("cargo");
    cmd.current_dir(module_root)
        .args(["build", "--release", "--target", "wasm32-unknown-unknown"]);
    ui::command("compiling wasm32-unknown-unknown (release)", &mut cmd)
        .context("cargo build wasm32")
}

/// devapi rend du camelCase, comme toutes les API Portaki. Sans ce rename, `size_bytes` ne
/// trouvait rien et le déploiement échouait à la lecture de sa propre réponse — alors qu'il
/// avait réussi côté serveur.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeployResponse {
    digest: String,
    size_bytes: u64,
}

/// `session` est celle du bail, quand nous l'avons obtenu.
///
/// Elle dit au serveur que ce push est celui du détenteur. Sans elle — bail non obtenu — il
/// n'admet le push que si personne d'autre ne tient la place, ce qui est exactement la garantie
/// que le verrou local ne peut pas donner.
async fn deploy(
    base_url: &str,
    module_id: &str,
    token: &str,
    wasm: &[u8],
    manifest: &str,
    session: Option<&str>,
) -> Result<DeployResponse> {
    let mut form = reqwest::multipart::Form::new()
        .part(
            "wasm",
            reqwest::multipart::Part::bytes(wasm.to_vec()).file_name("backend.wasm"),
        )
        .text("manifest", manifest.to_owned());
    if let Some(session) = session {
        form = form.text("sessionId", session.to_owned());
    }

    let response = reqwest::Client::new()
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

/// Même remarque, en pire : les `serde(default)` ci-dessous avalaient la non-correspondance en
/// silence. `--dispatch` affichait un résultat vide, une durée nulle et aucun host call sur une
/// invocation qui avait parfaitement tourné.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DispatchResponse {
    #[serde(default)]
    result_json: String,
    #[serde(default)]
    duration_ms: u64,
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
    let response = reqwest::Client::new()
        .post(format!("{}/dev/v1/modules/{module_id}/dispatch", base_url))
        .bearer_auth(token)
        .json(&body)
        .send()
        .await
        .context("dispatch on the dev platform")?;

    read_json(response).await
}

/// Prints what the run did — and what the sandbox refused to do.
fn print_trace(trace: &DispatchResponse) {
    if !trace.host_calls.is_empty() || !trace.captured_effects.is_empty() {
        ui::detail("what the run asked the host for:");
    }
    for call in &trace.host_calls {
        let outcome = if call.error_code.is_empty() {
            String::new()
        } else {
            format!("  ← {}", call.error_code)
        };
        ui::detail(format!(
            "{:>7} µs  {}{}",
            call.duration_micros, call.op, outcome
        ));
    }
    for effect in &trace.captured_effects {
        ui::detail(format!("captured  {}  {}", effect.op, effect.detail_json));
    }
    for event in &trace.published_events {
        ui::detail(format!("would publish  {event}"));
    }
    // « captured » et « would publish » se ressemblent assez pour qu'on les prenne pour des
    // choses faites. Elles ne le sont pas : la sandbox les note et les retient.
    if !trace.captured_effects.is_empty() || !trace.published_events.is_empty() {
        ui::detail("captured and would-publish lines were held, not performed");
    }
    if !trace.result_json.is_empty() {
        ui::result(&trace.result_json);
    }
}

/// Le seul échec dont on sait quoi faire : renouveler et rejouer. Il porte un type pour que
/// l'appelant le distingue d'un 500, qu'il serait absurde de rejouer avec un autre jeton.
#[derive(Debug)]
struct Unauthorized;

impl std::fmt::Display for Unauthorized {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the dev platform refused the token")
    }
}

impl std::error::Error for Unauthorized {}

async fn read_json<T: serde::de::DeserializeOwned>(response: reqwest::Response) -> Result<T> {
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

/// `--url`, then `PORTAKI_DEV_URL`, then `PORTAKI_API_URL`, then production.
fn base_url(args: &DevArgs) -> String {
    resolve_base_url(
        args.url.as_deref(),
        std::env::var("PORTAKI_DEV_URL").ok().as_deref(),
        std::env::var("PORTAKI_API_URL").ok().as_deref(),
    )
}

/// The precedence itself, free of the environment so it can be tested without touching it.
///
/// `PORTAKI_API_URL` is the variable `login` and `publish` read. Pointing at a staging platform
/// is a per-shell decision that applies to all three, and honouring it in only two sent `dev` to
/// production with a token minted elsewhere — a DNS failure at best, the wrong platform at
/// worst. `PORTAKI_DEV_URL` still wins, for the rarer case of a sandbox that lives apart.
///
/// An exported-but-empty variable means "unset", not "use the empty string as a URL".
fn resolve_base_url(
    explicit: Option<&str>,
    dev_var: Option<&str>,
    api_var: Option<&str>,
) -> String {
    let candidate = [explicit, dev_var, api_var]
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|value| !value.is_empty())
        .unwrap_or("https://api.portaki.app");
    candidate.trim_end_matches('/').to_string()
}

fn read_module_id(module_root: &Path) -> Result<String> {
    let manifest = module_root.join(MANIFEST);
    let raw = std::fs::read_to_string(&manifest)
        .with_context(|| format!("read {} — run from the module root", manifest.display()))?;
    let parsed: serde_json::Value =
        serde_json::from_str(&raw).context("parse portaki.module.json")?;
    parsed
        .get("id")
        .and_then(|id| id.as_str())
        .map(str::to_owned)
        .context("portaki.module.json carries no id")
}

/// Ce qui décide qu'un cycle a quelque chose à envoyer : le binaire et le manifeste ensemble.
fn upload_fingerprint(wasm: &[u8], manifest: &str) -> String {
    sha256(&[wasm, b"\0", manifest.as_bytes()].concat())
}

fn sha256(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

/// Digests are unreadable in full; the first bytes are enough to tell two builds apart.
fn short(digest: &str) -> String {
    digest.chars().take("sha256:".len() + 12).collect()
}

#[cfg(test)]
mod tests {

    /// Un manifeste modifié sans toucher au code doit repartir : c'est ce que `--watch` surveille.
    #[test]
    fn a_manifest_change_alone_is_something_to_upload() {
        let wasm = b"\0asm same bytes";
        assert_ne!(
            upload_fingerprint(wasm, r#"{"version":"0.4.0"}"#),
            upload_fingerprint(wasm, r#"{"version":"0.4.1"}"#)
        );
        assert_eq!(
            upload_fingerprint(wasm, "{}"),
            upload_fingerprint(wasm, "{}")
        );
    }

    use super::*;

    const PROD: &str = "https://api.portaki.app";

    /// Valeur obtenue par `printf '\0asm' | shasum -a 256`, pas recopiée de la sortie du test.
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

    /// La charge exacte que devapi renvoie, recopiée d'un déploiement réel.
    ///
    /// C'est le test qui manquait : la structure attendait `size_bytes`, la réponse portait
    /// `sizeBytes`, et le déploiement échouait à lire sa propre réussite.
    #[test]
    fn a_deploy_response_is_read_as_devapi_writes_it() {
        let body = r#"{"moduleId":"access-guide","version":"0.3.2",
            "digest":"sha256:3e9fc0c17866a50a005799fc53c854e696a101a9a6be0884b701cf157b4a9d1a",
            "permissions":["email","kv","platform"],"sizeBytes":1024374,
            "deployedAt":"2026-09-09T10:21:22.119744997Z"}"#;

        let parsed: DeployResponse = serde_json::from_str(body).expect("réponse de deploy lisible");

        assert_eq!(parsed.size_bytes, 1_024_374);
        assert!(parsed.digest.starts_with("sha256:"));
    }

    /// Les `serde(default)` d'une réponse de dispatch avalent une non-correspondance en silence :
    /// sans assertion sur les valeurs, un test de désérialisation passerait sur du vide.
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

    /// `PORTAKI_API_URL` est la variable que lisent `login` et `publish`. `dev` l'ignorait, et
    /// partait en production avec un jeton émis ailleurs.
    #[test]
    fn falls_back_to_the_shared_api_variable() {
        assert_eq!(
            resolve_base_url(None, None, Some("https://api-staging.portaki.app")),
            "https://api-staging.portaki.app"
        );
    }

    /// La sandbox peut vivre à part : sa variable dédiée reste prioritaire.
    #[test]
    fn prefers_the_dedicated_variable() {
        assert_eq!(
            resolve_base_url(
                None,
                Some("https://sandbox.example"),
                Some("https://api.example")
            ),
            "https://sandbox.example"
        );
    }

    /// `--url` l'emporte sur tout, et la barre finale ne double jamais celle du chemin.
    #[test]
    fn prefers_the_flag_and_trims_the_trailing_slash() {
        assert_eq!(
            resolve_base_url(
                Some("https://explicit.example/"),
                Some("https://ignored.example"),
                None
            ),
            "https://explicit.example"
        );
    }

    /// Une variable exportée vide vaut « non définie », pas « URL vide ».
    #[test]
    fn ignores_empty_and_blank_variables() {
        assert_eq!(resolve_base_url(None, Some(""), Some("   ")), PROD);
        assert_eq!(resolve_base_url(Some(""), None, None), PROD);
    }

    #[test]
    fn falls_back_to_production_when_nothing_is_set() {
        assert_eq!(resolve_base_url(None, None, None), PROD);
    }
}
