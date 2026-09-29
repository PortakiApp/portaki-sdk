//! `portaki doctor` — what, on this machine, would stop you from developing or publishing.
//!
//! One line per control, each with its fix. Nothing is pushed and nothing is signed: the registry
//! is asked for a push token, the toolchain for its target, cosign for its version.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Result;
use clap::Parser;
use serde_json::json;

use crate::api::Platform;
use crate::workspace::ModuleArgs;
use crate::{sign, ui};

#[derive(Debug, Parser)]
/// Arguments for `portaki doctor`.
pub struct DoctorArgs {
    #[command(flatten)]
    pub modules: ModuleArgs,
    /// Skip the lookups on crates.io and the registry — for an offline machine.
    #[arg(long)]
    pub offline: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
enum Status {
    Ok,
    /// A signal: nothing is blocked today.
    Warn,
    Fail,
    /// Not checkable here — the fix says how to check it.
    Skip,
}

#[derive(Debug, Clone, serde::Serialize)]
struct Check {
    id: &'static str,
    module: Option<String>,
    status: Status,
    summary: String,
    fix: Option<String>,
}

fn check(id: &'static str, status: Status, summary: impl Into<String>) -> Check {
    Check {
        id,
        module: None,
        status,
        summary: summary.into(),
        fix: None,
    }
}

impl Check {
    fn fix(mut self, fix: impl Into<String>) -> Self {
        self.fix = Some(fix.into());
        self
    }
}

/// Runs `portaki doctor`.
pub async fn run(args: DoctorArgs) -> Result<()> {
    ui::header(
        "portaki doctor",
        &crate::tr!("What on this machine would stop you from developing or publishing — and the fix.", "Ce qui, sur cette machine, empêcherait de développer ou de publier — et la correction."),
    );
    let base = crate::profile::api_url(None);
    let mut checks = Vec::new();

    let (session, mut platform) = session(&base).await;
    checks.push(session);
    checks.push(cli_version(args.offline).await);
    checks.push(cosign());
    checks.push(signing_identity(platform.as_ref()));

    let members = args.modules.resolve()?;
    for member in members.iter().filter(|member| !member.id.is_empty()) {
        let mut module = vec![
            toolchain(&member.root),
            sdk_versions(&member.root, args.offline).await,
            link(platform.as_mut(), &member.id).await,
            cargo_overrides(&member.root),
        ];
        for check in &mut module {
            check.module = Some(member.id.clone());
        }
        checks.extend(module);
    }
    if members.iter().all(|member| member.id.is_empty()) {
        checks.push(toolchain(&std::env::current_dir()?));
    }

    let failed = checks
        .iter()
        .filter(|check| check.status == Status::Fail)
        .count();
    render(&base, &checks);
    if failed > 0 {
        anyhow::bail!(
            "{failed} check{} failed",
            if failed == 1 { "" } else { "s" }
        );
    }
    Ok(())
}

fn render(base: &str, checks: &[Check]) {
    if ui::json() {
        ui::emit(&json!({
            "schemaVersion": 1,
            "api": base,
            "ok": checks.iter().all(|check| check.status != Status::Fail),
            "checks": checks,
        }));
        return;
    }
    ui::field("api", base);
    let mut current = None;
    for check in checks {
        if current != Some(check.module.as_deref()) {
            current = Some(check.module.as_deref());
            ui::section(check.module.as_deref().unwrap_or("this machine"));
        }
        let line = format!("{:<10} {}", check.id, check.summary);
        match check.status {
            Status::Ok => ui::success(line),
            Status::Warn => ui::warn(line),
            Status::Fail => ui::failure(line),
            Status::Skip => ui::skipped(line),
        }
        if let Some(fix) = &check.fix {
            ui::detail(format!("→ {fix}"));
        }
    }
    ui::blank();
}

/// A session for the origin being aimed at, and that it really opens something.
async fn session(base: &str) -> (Check, Option<Platform>) {
    let login = crate::profile::login_command(base);
    let mut platform = match Platform::open(base) {
        Ok(platform) => platform,
        Err(failure) => {
            return (
                check("session", Status::Fail, format!("{failure:#}")).fix(login),
                None,
            )
        }
    };
    match platform.get("/dev/v1/onboarding").await {
        Ok(_) => (
            check("session", Status::Ok, format!("signed in to {base}")),
            Some(platform),
        ),
        Err(failure) => (
            check(
                "session",
                Status::Fail,
                format!("the session for {base} does not open: {failure:#}"),
            )
            .fix(login),
            None,
        ),
    }
}

async fn cli_version(offline: bool) -> Check {
    let running = env!("CARGO_PKG_VERSION");
    if offline {
        return check("cli", Status::Skip, format!("portaki {running} (offline)"));
    }
    match crate::update::latest_published("portaki-cli").await {
        Ok(latest) if crate::update::outdated(running, &latest) => check(
            "cli",
            Status::Warn,
            format!("portaki {running} — {latest} is published"),
        )
        .fix("cargo install portaki-cli --locked --force"),
        Ok(_) => check("cli", Status::Ok, format!("portaki {running}, the latest")),
        Err(failure) => check(
            "cli",
            Status::Skip,
            format!("portaki {running} — could not reach crates.io: {failure}"),
        ),
    }
}

/// The toolchain `cargo` will pick in this directory, and its wasm32 target.
///
/// Through the sysroot rather than through `rustup`: a toolchain installed without rustup has its
/// target too, and it is this directory's `rustc` — `rust-toolchain.toml` included — that counts.
fn toolchain(dir: &Path) -> Check {
    let run = |args: &[&str]| {
        Command::new("rustc")
            .args(args)
            .current_dir(dir)
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
    };
    let Some(version) = run(&["--version"]) else {
        return check("toolchain", Status::Fail, "no rustc found")
            .fix("install Rust from https://rustup.rs");
    };
    let has_target = run(&["--print", "sysroot"])
        .map(|sysroot| {
            Path::new(&sysroot)
                .join("lib/rustlib/wasm32-unknown-unknown")
                .is_dir()
        })
        .unwrap_or(false);
    if has_target {
        check(
            "toolchain",
            Status::Ok,
            format!("{version} with wasm32-unknown-unknown"),
        )
    } else {
        check(
            "toolchain",
            Status::Fail,
            format!("{version} without the wasm32-unknown-unknown target"),
        )
        .fix("rustup target add wasm32-unknown-unknown")
    }
}

fn cosign() -> Check {
    let bin = sign::cosign_binary();
    let version = Command::new(&bin)
        .args(["version", "--json"])
        .output()
        .ok()
        .and_then(|output| serde_json::from_slice::<serde_json::Value>(&output.stdout).ok())
        .and_then(|version| Some(version.get("gitVersion")?.as_str()?.to_string()));
    match sign::check_cosign(&bin) {
        Ok(()) => check(
            "cosign",
            Status::Ok,
            format!("cosign {}", version.unwrap_or_default()),
        ),
        // `release` signs by default: without cosign you need `--no-sign`, and the version
        // will only ever run in the sandbox. A signal, not a blocker.
        Err(failure) => check("cosign", Status::Warn, format!("{failure:#}"))
            .fix("brew install cosign — portaki release signs with it"),
    }
}

/// The address Sigstore will certify has to be the verified Portaki address: otherwise the
/// registry refuses the author signature, after the push.
fn signing_identity(platform: Option<&Platform>) -> Check {
    if sign::in_ci() {
        return check(
            "signing",
            Status::Skip,
            "in CI, the release action signs with the workflow identity",
        );
    }
    let Some(portaki) = platform.and_then(|platform| verified_email(platform.token())) else {
        return check(
            "signing",
            Status::Skip,
            "no verified Portaki email in the session",
        )
        .fix("sign in, and verify your email in the developer space");
    };
    let how = format!(
        "Sigstore certifies the public email of your GitHub profile, or else its primary verified \
         email — make it {portaki} at https://github.com/settings/emails"
    );
    let Some(github) = github_email() else {
        return check(
            "signing",
            Status::Skip,
            format!("cannot read your GitHub email here (gh missing or signed out) — Portaki has {portaki}"),
        )
        .fix(how);
    };
    if github.eq_ignore_ascii_case(&portaki) {
        check(
            "signing",
            Status::Ok,
            format!("GitHub and Portaki both say {portaki}"),
        )
    } else {
        check(
            "signing",
            Status::Fail,
            format!("Sigstore would certify {github}, Portaki verified {portaki} — the registry would refuse the signature"),
        )
        .fix(how)
    }
}

/// `verified_email` from the access token — the very claim the registry compares. Read without
/// verifying the signature: this is a diagnostic, the registry checks it all again anyway.
fn verified_email(token: &str) -> Option<String> {
    use base64::Engine as _;
    let payload = token.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload.trim_end_matches('='))
        .ok()?;
    let claims: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    claims["verified_email"]
        .as_str()
        .filter(|email| !email.is_empty())
        .map(str::to_string)
}

/// The address Sigstore will see, read through `gh`: the profile's public address, otherwise the
/// primary verified one — the rule of Sigstore's GitHub connector.
fn github_email() -> Option<String> {
    let gh = std::env::var_os("PORTAKI_GH")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("gh"));
    let ask = |args: &[&str]| {
        Command::new(&gh)
            .args(args)
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
            .filter(|answer| !answer.is_empty() && answer != "null")
    };
    ask(&["api", "user", "--jq", ".email"]).or_else(|| {
        ask(&[
            "api",
            "user/emails",
            "--jq",
            ".[] | select(.primary and .verified) | .email",
        ])
    })
}

/// The CLI, the SDK the lock resolves, and the latest published SDK.
async fn sdk_versions(root: &Path, offline: bool) -> Check {
    use crate::commands::ci::{find_lockfile, read_locked_sdk};
    let cli = env!("CARGO_PKG_VERSION");
    let Some(locked) = find_lockfile(root)
        .and_then(|lock| std::fs::read_to_string(lock).ok())
        .and_then(|text| read_locked_sdk(&text))
    else {
        return check(
            "sdk",
            Status::Skip,
            "no Cargo.lock resolving portaki-sdk yet",
        )
        .fix("portaki build — cargo writes the lock");
    };
    let sdk = locked.version;
    if crate::update::outdated(cli, &sdk) {
        return check(
            "sdk",
            Status::Warn,
            format!("portaki {cli} is older than the SDK the module resolves ({sdk})"),
        )
        .fix(format!("cargo install portaki-cli@{sdk} --locked --force"));
    }
    if offline {
        return check(
            "sdk",
            Status::Ok,
            format!("portaki-sdk {sdk}, portaki {cli}"),
        );
    }
    match crate::update::latest_published("portaki-sdk").await {
        Ok(latest) if crate::update::outdated(&sdk, &latest) => check(
            "sdk",
            Status::Warn,
            format!("portaki-sdk {sdk} — {latest} is published"),
        )
        .fix("portaki upgrade"),
        Ok(_) => check(
            "sdk",
            Status::Ok,
            format!("portaki-sdk {sdk}, the latest; portaki {cli}"),
        ),
        Err(_) => check(
            "sdk",
            Status::Ok,
            format!("portaki-sdk {sdk}, portaki {cli} (crates.io unreachable)"),
        ),
    }
}

/// The GitHub environment that carries the stable channel's reviewers — decision of 28 Sept.
const STABLE_ENVIRONMENT: &str = "release";

async fn link(platform: Option<&mut Platform>, id: &str) -> Check {
    let Some(platform) = platform else {
        return check("link", Status::Skip, "needs a session");
    };
    match platform.get(&format!("/dev/v1/modules/{id}/link")).await {
        Ok(Some(link)) => {
            let repository = link["repository"].as_str().unwrap_or("a repository");
            match link["requiredEnvironment"].as_str() {
                Some(STABLE_ENVIRONMENT) => check(
                    "link",
                    Status::Ok,
                    format!("linked to {repository}, stable from the `release` environment"),
                ),
                other => check(
                    "link",
                    Status::Warn,
                    format!(
                        "linked to {repository}, but the stable channel's GitHub environment is {} — Portaki expects `release`",
                        other.map(|name| format!("`{name}`")).unwrap_or_else(|| "not set".to_string())
                    ),
                )
                .fix(format!(
                    "set the environment to `release` in the link (portaki link --module {id}), and `environment: release` on the release job"
                )),
            }
        }
        Ok(None) => check(
            "link",
            Status::Warn,
            "not linked to a repository — CI cannot publish it",
        )
        .fix(format!("portaki link --module {id}")),
        Err(failure) => check("link", Status::Skip, format!("{failure:#}")),
    }
}

/// What would make `cargo` take a toolchain, or options, other than the CLI's.
///
/// A signal, not a blocker: these files are often intended. But a module that carries one decides
/// what `cargo` runs — the publication job has to know about it.
fn cargo_overrides(root: &Path) -> Check {
    let found = overrides(root);
    if found.is_empty() {
        return check(
            "overrides",
            Status::Ok,
            "no .cargo/config or rust-toolchain redirects cargo",
        );
    }
    let names: Vec<String> = found
        .iter()
        .map(|path| path.display().to_string())
        .collect();
    check(
        "overrides",
        Status::Warn,
        format!("cargo reads {}", names.join(", ")),
    )
    .fix("make sure they are intended — they choose the toolchain and flags cargo runs with")
}

/// From the module up to the repository root (the directory that carries `.git`), included.
fn overrides(root: &Path) -> Vec<PathBuf> {
    const FILES: [&str; 4] = [
        ".cargo/config.toml",
        ".cargo/config",
        "rust-toolchain.toml",
        "rust-toolchain",
    ];
    let mut found = Vec::new();
    for dir in root.ancestors() {
        found.extend(
            FILES
                .iter()
                .map(|file| dir.join(file))
                .filter(|path| path.is_file()),
        );
        if dir.join(".git").exists() {
            break;
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_verified_email_is_read_from_the_token() {
        use base64::Engine as _;
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(r#"{"sub":"x","verified_email":"dev@example.com"}"#);
        assert_eq!(
            verified_email(&format!("h.{payload}.s")).as_deref(),
            Some("dev@example.com")
        );
        assert!(verified_email("not-a-jwt").is_none());
    }

    #[test]
    fn overrides_are_found_up_to_the_repository_root() {
        let repo = tempfile::tempdir().unwrap();
        let module = repo.path().join("modules/nuki");
        std::fs::create_dir_all(module.join(".cargo")).unwrap();
        std::fs::create_dir_all(repo.path().join(".git")).unwrap();
        std::fs::write(module.join(".cargo/config.toml"), "").unwrap();
        std::fs::write(repo.path().join("rust-toolchain.toml"), "").unwrap();

        let found = overrides(&module);
        assert_eq!(found.len(), 2, "{found:?}");
        assert_eq!(cargo_overrides(&module).status, Status::Warn);
        assert!(overrides(&repo.path().join("modules")).len() == 1);
    }
}
