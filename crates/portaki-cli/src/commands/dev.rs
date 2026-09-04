//! `portaki dev` — build, push to the hosted sandbox, and show what happened.
//!
//! Deliberately **not** a local gateway. A parallel engine always drifts from the real host, so
//! the module runs against the actual runtime in the sandbox; the difference with running
//! locally is latency, not nature — and no line of code leaves the infrastructure.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use clap::Parser;
use notify::{RecursiveMode, Watcher};
use sha2::{Digest as _, Sha256};

/// How long to wait for the editor to finish writing before rebuilding.
const DEBOUNCE: Duration = Duration::from_millis(300);

#[derive(Debug, Parser)]
/// Arguments for `portaki dev`.
pub struct DevArgs {
    /// Rebuild and redeploy on every save.
    #[arg(long)]
    pub watch: bool,

    /// Base URL of the dev platform. Defaults to PORTAKI_DEV_URL, then production.
    #[arg(long)]
    pub url: Option<String>,

    /// Operation to dispatch after each deploy. Omitted, the module is only deployed.
    #[arg(long)]
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
    let module_root = std::env::current_dir().context("current_dir")?;
    let mut token = crate::auth::access_token()?;
    let module_id = read_module_id(&module_root)?;
    let base_url = base_url(&args);

    let mut last_digest = String::new();
    cycle(
        &args,
        &base_url,
        &module_root,
        &module_id,
        &mut token,
        &mut last_digest,
    )
    .await?;

    if !args.watch {
        return Ok(());
    }

    let src = module_root.join("src");
    println!(
        "\nwatching {} — ⌘S to rebuild, ctrl-c to stop",
        src.display()
    );

    let (tx, rx) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |event| {
        let _ = tx.send(event);
    })
    .context("start file watcher")?;
    watcher
        .watch(&src, RecursiveMode::Recursive)
        .with_context(|| format!("watch {}", src.display()))?;

    loop {
        // Bloque jusqu'à la première sauvegarde…
        if rx.recv().is_err() {
            return Ok(());
        }
        // …puis absorbe la rafale qu'un éditeur produit en écrivant un fichier.
        while rx.recv_timeout(DEBOUNCE).is_ok() {}

        if let Err(failure) = cycle(
            &args,
            &base_url,
            &module_root,
            &module_id,
            &mut token,
            &mut last_digest,
        )
        .await
        {
            // Une erreur de compilation ne doit pas arrêter la boucle : c'est le cas courant.
            eprintln!("\n{failure:#}");
        }
    }
}

/// One pass: build, upload if it changed, optionally dispatch.
async fn cycle(
    args: &DevArgs,
    base_url: &str,
    module_root: &Path,
    module_id: &str,
    token: &mut String,
    last_digest: &mut String,
) -> Result<()> {
    build(module_root)?;

    let wasm_path = wasm_path(module_root, module_id);
    let wasm = std::fs::read(&wasm_path)
        .with_context(|| format!("read {} — did the build produce it?", wasm_path.display()))?;
    let digest = sha256(&wasm);

    if digest == *last_digest {
        println!("unchanged ({}), nothing to upload", short(&digest));
        return Ok(());
    }

    let manifest = std::fs::read_to_string(module_root.join("portaki.module.json"))
        .context("read portaki.module.json")?;

    // Le résultat est lié avant le match : garder l'appel comme sujet du match retiendrait
    // l'emprunt du jeton pendant qu'on cherche à le remplacer.
    let first = deploy(base_url, module_id, token, &wasm, &manifest).await;
    let deployed = match first {
        Err(failure) if failure.is::<Unauthorized>() => {
            *token = reauthenticate().await?;
            deploy(base_url, module_id, token, &wasm, &manifest).await?
        }
        other => other?,
    };
    println!(
        "deployed {} {} — {} bytes",
        module_id,
        short(&deployed.digest),
        deployed.size_bytes
    );
    *last_digest = digest;

    if let Some(operation) = &args.dispatch {
        let first = dispatch(args, base_url, module_id, token, operation).await;
        let trace = match first {
            Err(failure) if failure.is::<Unauthorized>() => {
                *token = reauthenticate().await?;
                dispatch(args, base_url, module_id, token, operation).await?
            }
            other => other?,
        };
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
    println!("access token expired — renewing");
    crate::auth::refresh()
        .await
        .context("renew the session — run `portaki login` if this keeps failing")
}

fn build(module_root: &Path) -> Result<()> {
    // Release, pas debug : un build debug pèse dix fois plus et se fait refuser par le plafond
    // d'ingestion de 5 Mo. Mieux vaut compiler plus longtemps que découvrir le refus au push.
    let status = std::process::Command::new("cargo")
        .current_dir(module_root)
        .args(["build", "--release", "--target", "wasm32-unknown-unknown"])
        .status()
        .context("cargo build wasm32")?;
    if !status.success() {
        bail!("cargo build failed");
    }
    Ok(())
}

fn wasm_path(module_root: &Path, module_id: &str) -> PathBuf {
    module_root
        .join("target/wasm32-unknown-unknown/release")
        .join(format!("{module_id}.wasm"))
}

#[derive(Debug, serde::Deserialize)]
struct DeployResponse {
    digest: String,
    size_bytes: u64,
}

async fn deploy(
    base_url: &str,
    module_id: &str,
    token: &str,
    wasm: &[u8],
    manifest: &str,
) -> Result<DeployResponse> {
    let form = reqwest::multipart::Form::new()
        .part(
            "wasm",
            reqwest::multipart::Part::bytes(wasm.to_vec()).file_name("backend.wasm"),
        )
        .text("manifest", manifest.to_owned());

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

#[derive(Debug, serde::Deserialize)]
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
struct HostCall {
    op: String,
    duration_micros: u64,
    #[serde(default)]
    error_code: String,
}

#[derive(Debug, serde::Deserialize)]
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
    println!("  {} ms", trace.duration_ms);
    for call in &trace.host_calls {
        let outcome = if call.error_code.is_empty() {
            String::new()
        } else {
            format!("  ← {}", call.error_code)
        };
        println!("  {:>7} µs  {}{}", call.duration_micros, call.op, outcome);
    }
    for effect in &trace.captured_effects {
        println!("  captured  {}  {}", effect.op, effect.detail_json);
    }
    for event in &trace.published_events {
        println!("  would publish  {event}");
    }
    if !trace.result_json.is_empty() {
        println!("  → {}", trace.result_json);
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

/// `--url`, then `PORTAKI_DEV_URL`, then production.
fn base_url(args: &DevArgs) -> String {
    let raw = args
        .url
        .clone()
        .or_else(|| std::env::var("PORTAKI_DEV_URL").ok())
        .unwrap_or_else(|| "https://api.portaki.app".to_string());
    raw.trim_end_matches('/').to_string()
}

fn read_module_id(module_root: &Path) -> Result<String> {
    let manifest = module_root.join("portaki.module.json");
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

fn sha256(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

/// Digests are unreadable in full; the first bytes are enough to tell two builds apart.
fn short(digest: &str) -> String {
    digest.chars().take("sha256:".len() + 12).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
