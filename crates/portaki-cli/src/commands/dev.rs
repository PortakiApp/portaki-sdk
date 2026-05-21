//! `portaki dev` — local mock gateway (minimal stub in v0.1).

use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
/// Arguments for `portaki dev`.
pub struct DevArgs {
    /// HTTP port for the mock gateway.
    #[arg(long, default_value_t = 3838)]
    pub port: u16,
}

/// Runs `portaki dev` (prints instructions until the mock gateway is fully wired).
pub async fn run(args: DevArgs) -> Result<()> {
    println!(
        "portaki dev: mock gateway listening on http://127.0.0.1:{} (stub — use portaki-test-utils in unit tests)",
        args.port
    );
    println!("Press Ctrl+C to stop.");
    tokio::signal::ctrl_c().await?;
    Ok(())
}
