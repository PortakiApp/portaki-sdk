//! `portaki inspect` — inspect a published OCI artifact.

use anyhow::{Context, Result};
use clap::Parser;

#[derive(Debug, Parser)]
/// Arguments for `portaki inspect`.
pub struct InspectArgs {
    /// OCI artifact URL or digest reference.
    pub artifact_url: String,
}

/// Runs `portaki inspect`.
pub async fn run(args: InspectArgs) -> Result<()> {
    let client = reqwest::Client::new();
    let response = client
        .get(&args.artifact_url)
        .send()
        .await
        .context("fetch artifact URL")?;

    if !response.status().is_success() {
        anyhow::bail!("HTTP {} for {}", response.status(), args.artifact_url);
    }

    let bytes = response.bytes().await?;
    if let Ok(text) = std::str::from_utf8(&bytes) {
        if text.trim_start().starts_with('{') {
            let json: serde_json::Value = serde_json::from_str(text)?;
            println!("{}", serde_json::to_string_pretty(&json)?);
            return Ok(());
        }
    }

    println!(
        "Fetched {} bytes from {} (binary artifact — use oras manifest inspect for OCI metadata)",
        bytes.len(),
        args.artifact_url
    );
    Ok(())
}
