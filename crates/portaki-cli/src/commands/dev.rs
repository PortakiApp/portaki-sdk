//! `portaki dev` — local mock gateway (not implemented yet).

use anyhow::{bail, Result};
use clap::Parser;

#[derive(Debug, Parser)]
/// Arguments for `portaki dev`.
pub struct DevArgs {
    /// HTTP port that would be used once the mock gateway lands.
    #[arg(long, default_value_t = 3838)]
    pub port: u16,
}

/// Runs `portaki dev`.
///
/// Fails clearly until a real local gateway exists — does not pretend to listen.
pub async fn run(args: DevArgs) -> Result<()> {
    bail!(
        "portaki dev is not implemented yet (requested port {}). \
         Use portaki-test-utils::MockContext in unit tests instead.",
        args.port
    );
}
