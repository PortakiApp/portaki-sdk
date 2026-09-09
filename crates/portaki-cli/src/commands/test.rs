//! `portaki test` — cargo test wrapper.

use std::process::Command;

use anyhow::{Context, Result};
use clap::Parser;

use crate::ui;

#[derive(Debug, Parser)]
/// Arguments for `portaki test`.
pub struct TestArgs {
    /// Extra arguments forwarded to `cargo test`.
    #[arg(last = true)]
    pub cargo_args: Vec<String>,
}

/// Runs `portaki test`.
pub fn run(args: TestArgs) -> Result<()> {
    ui::header("portaki test");

    let mut cmd = Command::new("cargo");
    cmd.arg("test");
    for arg in &args.cargo_args {
        cmd.arg(arg);
    }

    // La sortie de `cargo test` est le sujet de la commande : elle passe en direct, sans
    // indicateur pour la masquer.
    let status = cmd.status().context("cargo test")?;
    ui::blank();
    if status.success() {
        ui::success("tests passed");
        ui::blank();
        Ok(())
    } else {
        anyhow::bail!("cargo test failed");
    }
}
