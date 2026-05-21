//! `portaki` CLI — module authoring toolchain.

mod commands;
mod manifest;
mod oci;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(name = "portaki", version, about = "Portaki module SDK CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Scaffold a new module from a template.
    Init(commands::init::InitArgs),
    /// Local mock gateway + file watcher (stub in v0.1).
    Dev(commands::dev::DevArgs),
    /// Build wasm32 artifact, manifest, and i18n bundle.
    Build(commands::build::BuildArgs),
    /// Validate manifest, i18n keys, and capability ids.
    Lint(commands::lint::LintArgs),
    /// Run `cargo test` in the module crate.
    Test(commands::test::TestArgs),
    /// Push OCI artifact to Scaleway Container Registry.
    Publish(commands::publish::PublishArgs),
    /// Open local SDK documentation (prints instructions in v0.1).
    Docs(commands::docs::DocsArgs),
    /// Dump the SDUI catalog specification.
    Catalog(commands::catalog::CatalogArgs),
    /// Inspect a published OCI artifact URL.
    Inspect(commands::inspect::InspectArgs),
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    match cli.command {
        Command::Init(args) => commands::init::run(args),
        Command::Dev(args) => commands::dev::run(args).await,
        Command::Build(args) => commands::build::run(args).await,
        Command::Lint(args) => commands::lint::run(args),
        Command::Test(args) => commands::test::run(args),
        Command::Publish(args) => commands::publish::run(args).await,
        Command::Docs(args) => commands::docs::run(args),
        Command::Catalog(args) => commands::catalog::run(args),
        Command::Inspect(args) => commands::inspect::run(args).await,
    }
}
