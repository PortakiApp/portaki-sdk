//! `portaki docs` — local documentation server hint.

use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
/// Arguments for `portaki docs`.
pub struct DocsArgs {}

/// Runs `portaki docs`.
pub fn run(_args: DocsArgs) -> Result<()> {
    println!("Generate Rust API docs with:");
    println!("  cargo doc --workspace --no-deps --open");
    println!("SDUI catalog: portaki catalog --format json");
    Ok(())
}
