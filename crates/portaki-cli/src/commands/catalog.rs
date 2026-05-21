//! `portaki catalog` — dump SDUI primitive catalog.

use anyhow::Result;
use clap::{Parser, ValueEnum};
use std::fs;

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
    #[arg(long, value_enum, default_value_t = CatalogFormat::Json)]
    pub format: CatalogFormat,
}

/// Runs `portaki catalog`.
pub fn run(args: CatalogArgs) -> Result<()> {
    let catalog_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../portaki-sdk/sdui_primitives.json");
    let text = fs::read_to_string(&catalog_path).unwrap_or_else(|_| "[]".to_string());

    match args.format {
        CatalogFormat::Json => {
            let value: serde_json::Value = serde_json::from_str(&text)?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
    }

    Ok(())
}
