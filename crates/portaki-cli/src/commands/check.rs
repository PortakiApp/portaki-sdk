//! `portaki check` — the gate a CI runs, in one command.

use std::process::Command;

use anyhow::{Context, Result};
use clap::Parser;

use crate::commands::build::{self, BuildArgs};
use crate::commands::lint::{self, LintArgs};
use crate::ui;

#[derive(Debug, Parser)]
/// Arguments for `portaki check`.
pub struct CheckArgs {
    /// Fix what can be fixed — `cargo fmt`, then `cargo clippy --fix` — instead of refusing.
    #[arg(long)]
    pub fix: bool,
    /// Skip `cargo test`.
    #[arg(long)]
    pub skip_tests: bool,
}

/// Runs `portaki check`.
///
/// The order is the one that fails fastest: formatting before lints, lints before tests, and
/// the wasm build last, since it is the one that takes ten seconds.
pub async fn run(args: CheckArgs) -> Result<()> {
    ui::header(
        "portaki check",
        "Formatting, lints, tests, the wasm build, and the manifest — what CI will ask.",
    );
    let started = std::time::Instant::now();

    format_step(args.fix)?;
    clippy_step(args.fix)?;
    if args.skip_tests {
        ui::skipped("tests skipped (--skip-tests)");
    } else {
        let mut cmd = Command::new("cargo");
        cmd.arg("test");
        ui::command("cargo test", &mut cmd).context("cargo test")?;
    }

    ui::blank();
    build::run(BuildArgs {
        release: true,
        manifest_only: false,
        nested: true,
    })
    .await?;
    lint::run(LintArgs {
        manifest: None,
        nested: true,
    })?;

    ui::blank();
    ui::success(format!("checked in {}", ui::elapsed(started.elapsed())));
    ui::next(&[
        (
            "portaki dev --watch",
            "run it in the hosted sandbox on every save",
        ),
        ("portaki publish", "push the artifact and announce it"),
    ]);
    ui::blank();
    Ok(())
}

fn format_step(fix: bool) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.arg("fmt");
    if !fix {
        cmd.arg("--check");
    }
    let label = if fix {
        "cargo fmt"
    } else {
        "cargo fmt --check"
    };
    ui::command(label, &mut cmd).context(label.to_string())
}

/// Clippy as CI runs it: every target, and a warning is a failure.
///
/// With `--fix`, the fixing pass runs first and the strict pass still runs after it. `cargo
/// clippy --fix` returns success on what it could not rewrite — dead code, for one — so trusting
/// it alone would report a module clean that CI is about to refuse.
fn clippy_step(fix: bool) -> Result<()> {
    if fix {
        let mut fixing = Command::new("cargo");
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
    cmd.arg("clippy")
        .arg("--all-targets")
        .arg("--")
        .arg("-D")
        .arg("warnings");
    ui::command("cargo clippy -D warnings", &mut cmd).context("cargo clippy -D warnings")
}
