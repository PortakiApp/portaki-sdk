//! `portaki catalog` — dump SDUI primitive catalog.

use anyhow::Result;
use clap::{Parser, ValueEnum};

#[derive(Debug, Clone, ValueEnum)]
/// Output format for catalog dump.
pub enum CatalogFormat {
    /// Pretty-printed JSON.
    Json,
}

#[derive(Debug, Parser)]
/// Arguments for `portaki catalog`.
pub struct CatalogArgs {
    /// Output format.
    // Hidden while JSON is the only one: a flag with a single value reads as a choice that
    // isn't. Still accepted, so `--format json` in a script keeps working.
    #[arg(long, value_enum, default_value_t = CatalogFormat::Json, hide = true)]
    pub format: CatalogFormat,
}

/// Runs `portaki catalog`.
pub fn run(args: CatalogArgs) -> Result<()> {
    // Read from the SDK crate this binary was linked against, not from a sibling directory of
    // the source tree: installed from crates.io, that path resolved to nothing and the command
    // printed `[]` — an empty catalogue that `portaki docs` calls the documentation.
    let text = portaki_sdk::SDUI_PRIMITIVES_JSON;

    match args.format {
        CatalogFormat::Json => {
            let value: serde_json::Value = serde_json::from_str(text)?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
    }

    Ok(())
}
