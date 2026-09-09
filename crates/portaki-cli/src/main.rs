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

mod api;
mod auth;
mod commands;
mod manifest;
mod oci;
mod oidc;
mod ui;

use anyhow::Result;
use clap::builder::styling::{AnsiColor, Effects, Styles};
use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};
use tracing_subscriber::EnvFilter;

/// L'aide de `clap` peinte comme le reste de la sortie : un seul vocabulaire visuel, que la
/// ligne vienne de `--help` ou d'une commande.
const HELP_STYLES: Styles = Styles::styled()
    .header(AnsiColor::Cyan.on_default().effects(Effects::BOLD))
    .usage(AnsiColor::Cyan.on_default().effects(Effects::BOLD))
    .literal(AnsiColor::White.on_default().effects(Effects::BOLD))
    .placeholder(AnsiColor::Cyan.on_default())
    .error(AnsiColor::Red.on_default().effects(Effects::BOLD))
    .invalid(AnsiColor::Yellow.on_default());

#[derive(Debug, Parser)]
#[command(
    name = "portaki",
    version,
    about = "Portaki module SDK CLI",
    styles = HELP_STYLES,
    arg_required_else_help = true
)]
struct Cli {
    /// Plain text only — no colour, no spinners.
    #[arg(long, global = true)]
    no_color: bool,

    /// Stream the raw output of the tools the CLI drives.
    #[arg(long, short, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Scaffold a new module from a template.
    Init(commands::init::InitArgs),
    /// Sign in with the device grant and store the token in the system keychain.
    Login(commands::login::LoginArgs),
    /// Clear the stored credentials.
    Logout,
    /// Build, push to the hosted sandbox, and show what the run did.
    Dev(commands::dev::DevArgs),
    /// Build wasm32 artifact, manifest, and i18n bundle.
    Build(commands::build::BuildArgs),
    /// Validate manifest, i18n keys, and capability ids.
    Lint(commands::lint::LintArgs),
    /// Run `cargo test` in the module crate.
    Test(commands::test::TestArgs),
    /// Push OCI artifact to Scaleway Container Registry.
    Publish(commands::publish::PublishArgs),
    /// Print how to open local SDK documentation (no docs server).
    Docs(commands::docs::DocsArgs),
    /// Dump the SDUI catalog specification.
    Catalog(commands::catalog::CatalogArgs),
    /// Inspect a published OCI artifact URL.
    Inspect(commands::inspect::InspectArgs),
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = parse();
    ui::init(cli.no_color, cli.verbose);

    // L'échec est rendu ici, une fois, au lieu du `Debug` que `main() -> Result` imprime : la
    // chaîne des causes se lit, et la sortie d'erreur ressemble au reste de la CLI.
    if let Err(failure) = dispatch(cli.command).await {
        ui::report(&failure);
        std::process::exit(1);
    }
}

/// Analyse les arguments, en habillant l'aide et `--version` de ce que `clap` ne sait pas seul.
///
/// `clap` rend ces deux écrans pendant l'analyse, donc avant qu'on ait lu le moindre argument :
/// `--no-color` est cherché à la main d'abord, sans quoi un logo en couleurs partirait dans un
/// fichier de sortie qu'on avait justement demandé nu.
fn parse() -> Cli {
    let wants_color = !std::env::args().any(|argument| argument == "--no-color");
    ui::set_colors(wants_color);

    let command = Cli::command()
        .before_help(ui::banner())
        .before_long_help(ui::banner())
        .after_help(ui::legal())
        .after_long_help(ui::legal())
        // `clap` veut une chaîne qui vit aussi longtemps que le programme ; celle-ci est
        // construite une fois, au démarrage, et l'écran de version en est le seul lecteur.
        .long_version(Box::leak(ui::long_version().into_boxed_str()) as &'static str);

    Cli::from_arg_matches(&command.get_matches()).unwrap_or_else(|failure| failure.exit())
}

async fn dispatch(command: Command) -> Result<()> {
    match command {
        Command::Init(args) => commands::init::run(args),
        Command::Login(args) => commands::login::run(args).await,
        Command::Logout => commands::login::logout(),
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
