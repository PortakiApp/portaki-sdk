//! `portaki docs` — prints how to open local API docs (no fake server).

use anyhow::Result;
use clap::Parser;

use crate::ui;

#[derive(Debug, Parser)]
/// Arguments for `portaki docs`.
pub struct DocsArgs {}

/// Prints documentation commands — does not start a docs server.
pub fn run(_args: DocsArgs) -> Result<()> {
    ui::header("portaki docs");
    ui::detail("no docs server — these two commands are the documentation");
    ui::next(&[
        "cargo doc --workspace --no-deps --open",
        "portaki catalog --format json",
    ]);
    ui::blank();
    Ok(())
}
