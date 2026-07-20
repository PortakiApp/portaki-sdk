//! `portaki` — command-line toolchain for Portaki Extism Wasm modules.
//!
//! # Role
//!
//! Authors write modules against [`portaki_sdk`]. At build time this binary:
//!
//! 1. Compiles the crate to `wasm32-unknown-unknown`
//! 2. Reads proc-macro JSON under `OUT_DIR/portaki-emissions/`
//! 3. Merges emissions (+ optional hand-written `portaki.module.json`) into `manifest.json`
//! 4. Packages Wasm + manifests for OCI push (`portaki publish`)
//!
//! # Commands
//!
//! | Command | Contract |
//! |---------|----------|
//! | `init` | Scaffold a module crate from a template |
//! | `build` | Produce Wasm + merged manifest (+ migrations/operations bundles + i18n) |
//! | `lint` | Validate capability ids, connector bindings, i18n keys |
//! | `test` | Forward to `cargo test` in the module crate |
//! | `publish` | Push OCI layers to a container registry |
//! | `catalog` | Dump the SDUI primitive catalog the host understands |
//! | `inspect` | Fetch and summarize a published OCI artifact |
//! | `docs` / `dev` | Docs helper / local mock gateway (evolve with the SDK) |
//!
//! Install: `cargo install portaki-cli`. Requires `rustup target add wasm32-unknown-unknown`.

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
