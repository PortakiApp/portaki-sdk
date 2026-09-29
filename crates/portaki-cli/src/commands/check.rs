//! `portaki check` — the gate of `portaki release`, and nothing else.
//!
//! [`gate`] is called exactly as it stands by `release` (and `ci build`) before anything is
//! pushed: what `check` lets through, `release` lets through, and the other way round. The
//! controls, in the order that fails fastest: format, clippy, tests (the conformance battery
//! included), the wasm `--release` build, the manifest (`lint`), the texts of every language
//! (`i18n`).

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};

use crate::commands::build::{self, BuildArgs};
use crate::commands::{i18n, lint, test};
use crate::ui;

/// One control of the gate, as `--only` names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Control {
    /// `cargo fmt --check`
    Fmt,
    /// `cargo clippy --all-targets -- -D warnings`
    Clippy,
    /// `cargo test`, the conformance battery included
    Tests,
    /// The wasm32 build, `--release`
    Build,
    /// The manifest: capability ids, connector bindings, i18n keys, SDK version, feeds
    Lint,
    /// Every text in every language of `i18n/` and `email_i18n/`
    I18n,
}

/// The whole gate, in order.
pub const ALL: [Control; 6] = [
    Control::Fmt,
    Control::Clippy,
    Control::Tests,
    Control::Build,
    Control::Lint,
    Control::I18n,
];

#[derive(Debug, Parser)]
/// Arguments for `portaki check`.
pub struct CheckArgs {
    /// Fix what can be fixed — `cargo fmt`, then `cargo clippy --fix` — instead of refusing.
    #[arg(long)]
    pub fix: bool,
    /// Run only these controls (comma-separated).
    #[arg(long, value_enum, value_delimiter = ',', value_name = "CONTROL")]
    pub only: Vec<Control>,
    /// The channel the module is headed for: `stable` refuses an SDK older than 8.0.0.
    #[arg(long, default_value = "stable", value_parser = ["preview", "stable"])]
    pub channel: String,
    /// Former flag — now `--only fmt,clippy,build,lint,i18n`.
    #[arg(long, hide = true)]
    pub skip_tests: bool,
    #[command(flatten)]
    pub modules: crate::workspace::ModuleArgs,
}

/// Runs `portaki check`.
pub async fn run(args: CheckArgs) -> Result<()> {
    ui::header(
        "portaki check",
        &crate::tr!(
            "The gate portaki release applies — the same controls, before anything is pushed.",
            "La porte que portaki release applique — les mêmes contrôles, avant toute poussée."
        ),
    );
    let mut only = args.only.clone();
    if args.skip_tests {
        ui::warn(crate::tr!(
            "--skip-tests is now --only fmt,clippy,build,lint,i18n — use that from now on",
            "--skip-tests devient --only fmt,clippy,build,lint,i18n — utilisez désormais cette forme"
        ));
        only = ALL.into_iter().filter(|c| *c != Control::Tests).collect();
    }
    run_controls(&args, &only).await
}

/// The `only` controls (all of them when empty), module after module; the first failure stops.
async fn run_controls(args: &CheckArgs, only: &[Control]) -> Result<()> {
    let started = std::time::Instant::now();
    let chosen = args.modules.resolve()?;
    let mut results = Vec::new();
    let mut failed = None;
    for member in &chosen {
        if chosen.len() > 1 {
            ui::rule(&member.id);
        }
        crate::workspace::enter(member)?;
        let outcome = gate(&member.root, &args.channel, args.fix, only)
            .await
            .with_context(|| format!("module {}", member.id));
        results.push(serde_json::json!({
            "id": member.id,
            "ok": outcome.is_ok(),
            "error": outcome.as_ref().err().map(|failure| format!("{failure:#}")),
        }));
        if let Err(failure) = outcome {
            failed = Some(failure);
            break;
        }
    }

    if ui::json() {
        ui::emit(&serde_json::json!({
            "schemaVersion": 1,
            "controls": if only.is_empty() { ALL.to_vec() } else { only.to_vec() },
            "modules": results,
        }));
    }
    if let Some(failure) = failed {
        return Err(failure);
    }
    ui::blank();
    if only.is_empty() {
        ui::success(crate::tr!(
            "checked in {} — portaki release will let it through",
            "vérifié en {} — portaki release le laissera passer",
            ui::elapsed(started.elapsed())
        ));
    } else {
        ui::success(crate::tr!(
            "checked in {} — part of the gate only (--only)",
            "vérifié en {} — une partie de la porte seulement (--only)",
            ui::elapsed(started.elapsed())
        ));
    }
    ui::next(&[(
        "portaki release",
        &crate::tr!(
            "push the artifact, sign it and announce it",
            "pousser l'artefact, le signer et l'annoncer"
        ),
    )]);
    ui::blank();
    Ok(())
}

/// The publication gate, for the module in `module_root` (the current directory).
///
/// An empty `only`: every control. `release` always calls it that way, without `--fix`.
pub async fn gate(module_root: &Path, channel: &str, fix: bool, only: &[Control]) -> Result<()> {
    let runs = |control: Control| only.is_empty() || only.contains(&control);
    if runs(Control::Fmt) {
        format_step(module_root, fix)?;
    }
    if runs(Control::Clippy) {
        clippy_step(module_root, fix)?;
    }
    if runs(Control::Tests) {
        test::gate_publish(module_root).context("tests before publish")?;
    }
    if runs(Control::Build) {
        ui::blank();
        build::run(BuildArgs {
            release: true,
            manifest_only: false,
            module: None,
            all: false,
            nested: true,
        })
        .await
        .context("portaki build --release")?;
    }
    if runs(Control::Lint) {
        lint::run(lint::LintArgs {
            manifest: None,
            channel: channel.to_string(),
            modules: Default::default(),
            nested: true,
        })
        .context("manifest")?;
    }
    if runs(Control::I18n) {
        i18n::gate(module_root)?;
    }
    Ok(())
}

fn format_step(module_root: &Path, fix: bool) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.current_dir(module_root).arg("fmt");
    if !fix {
        cmd.arg("--check");
    }
    let label = if fix {
        "cargo fmt"
    } else {
        "cargo fmt --check"
    };
    ui::command(label, &mut cmd).with_context(|| {
        crate::tr!(
            "{label} — portaki check --fix formats; rustfmt missing: rustup component add rustfmt",
            "{label} — portaki check --fix formate ; rustfmt absent : rustup component add rustfmt"
        )
    })
}

/// Clippy as CI runs it: every target, and a warning is a failure.
///
/// With `--fix`, the fixing pass runs first and the strict pass still runs after it. `cargo
/// clippy --fix` returns success on what it could not rewrite — dead code, for one — so trusting
/// it alone would report a module clean that CI is about to refuse.
fn clippy_step(module_root: &Path, fix: bool) -> Result<()> {
    if fix {
        let mut fixing = Command::new("cargo");
        fixing.current_dir(module_root);
        // A fix rewrites sources; cargo refuses to do that over uncommitted work unless told
        // that it is expected — and it is, since this is the flag that asks for it.
        fixing
            .arg("clippy")
            .arg("--fix")
            .arg("--allow-dirty")
            .arg("--allow-staged")
            .arg("--all-targets");
        ui::command("cargo clippy --fix", &mut fixing).context("cargo clippy --fix")?;
    }

    let mut cmd = Command::new("cargo");
    cmd.current_dir(module_root)
        .arg("clippy")
        .arg("--all-targets")
        .arg("--")
        .arg("-D")
        .arg("warnings");
    ui::command("cargo clippy -D warnings", &mut cmd).with_context(|| {
        crate::tr!(
            "cargo clippy -D warnings — clippy missing: rustup component add clippy",
            "cargo clippy -D warnings — clippy absent : rustup component add clippy"
        )
    })
}
