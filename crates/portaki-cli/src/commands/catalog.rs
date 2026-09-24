//! `portaki catalog` — dump SDUI primitive catalog.

use anyhow::Result;

/// Runs `portaki catalog`: prints the SDUI primitive catalog as pretty JSON.
pub fn run() -> Result<()> {
    // Read from the SDK crate this binary was linked against, not from a sibling directory of
    // the source tree: installed from crates.io, that path resolved to nothing and the command
    // printed `[]` — an empty catalogue that `portaki docs` calls the documentation.
    let value: serde_json::Value = serde_json::from_str(portaki_sdk::SDUI_PRIMITIVES_JSON)?;
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}
