//! `portaki docs` — prints how to open local API docs (no fake server).

use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
/// Arguments for `portaki docs`.
pub struct DocsArgs {}

/// Prints documentation commands — does not start a docs server.
pub fn run(_args: DocsArgs) -> Result<()> {
    println!("portaki docs does not serve pages locally.");
    println!("Generate Rust API docs with:");
    println!("  cargo doc --workspace --no-deps --open");
    println!("SDUI catalog: portaki catalog --format json");
    Ok(())
}
