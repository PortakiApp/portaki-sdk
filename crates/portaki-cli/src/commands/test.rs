//! `portaki test` — cargo test wrapper.

use std::process::Command;

use anyhow::{Context, Result};
use clap::Parser;

#[derive(Debug, Parser)]
/// Arguments for `portaki test`.
pub struct TestArgs {
    /// Extra arguments forwarded to `cargo test`.
    #[arg(last = true)]
    pub cargo_args: Vec<String>,
}

/// Runs `portaki test`.
pub fn run(args: TestArgs) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.arg("test");
    for arg in &args.cargo_args {
        cmd.arg(arg);
    }
    let status = cmd.status().context("cargo test")?;
    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("cargo test failed");
    }
}
