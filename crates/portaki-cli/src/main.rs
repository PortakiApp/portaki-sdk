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

    let matches = match command.clone().try_get_matches() {
        Ok(matches) => matches,
        Err(refusal) => refuse(refusal, &command),
    };
    Cli::from_arg_matches(&matches).unwrap_or_else(|failure| failure.exit())
}

/// Rend le refus de `clap` comme le reste de la CLI, et dit où chercher.
///
/// « a value is required for '--dispatch <DISPATCH>' » est exact et n'aide pas : il manque ce
/// qu'on aurait pu écrire. Quand le refus porte sur la commande elle-même, la liste des
/// commandes suit — c'est la seule réponse à « laquelle ? ».
fn refuse(refusal: clap::Error, command: &clap::Command) -> ! {
    use clap::error::ErrorKind;

    // `--help` et `--version` ne sont pas des échecs : `clap` les rend lui-même et sort en 0.
    if !refusal.use_stderr() {
        refusal.exit();
    }

    let rendered = refusal.to_string();
    ui::blank();
    ui::failure(headline(&rendered));

    // `clap` sait souvent proposer le nom qu'on visait ; le perdre serait retirer la seule
    // chose vraiment utile de son message.
    for tip in rendered
        .lines()
        .filter_map(|line| line.trim().strip_prefix("tip: "))
    {
        ui::detail(tip);
    }

    // La liste des commandes ne répond qu'à « laquelle ? ». Sur un flag inconnu, on est déjà
    // dans une commande : la dérouler entière serait du bruit devant la vraie question.
    if matches!(
        refusal.kind(),
        ErrorKind::InvalidSubcommand | ErrorKind::MissingSubcommand
    ) {
        // `get_about` rend un `StyledStr` : il faut le matérialiser avant d'en prêter des
        // tranches à la liste.
        let commands: Vec<(String, String)> = command
            .get_subcommands()
            .filter(|sub| !sub.is_hide_set())
            .map(|sub| {
                (
                    sub.get_name().to_string(),
                    sub.get_about().map(ToString::to_string).unwrap_or_default(),
                )
            })
            .collect();
        let rows: Vec<(&str, &str)> = commands
            .iter()
            .map(|(name, about)| (name.as_str(), about.as_str()))
            .collect();
        ui::list("commands", &rows);
    }

    let help = match invoked_command(command) {
        Some(name) => format!("portaki {name} --help"),
        None => "portaki --help".to_string(),
    };
    ui::next(&[(&help, "every flag this command takes")]);
    ui::blank();
    std::process::exit(2);
}

/// La première ligne du refus, sans le « error: » que `clap` préfixe — la croix le dit déjà.
fn headline(rendered: &str) -> String {
    rendered
        .lines()
        .find(|line| !line.trim().is_empty())
        .map(|line| line.trim_start_matches("error: ").to_string())
        .unwrap_or_else(|| "invalid arguments".to_string())
}

/// La sous-commande que la ligne de commande nommait, s'il y en avait une de connue.
///
/// Lue des arguments bruts : le refus est arrivé avant qu'aucune analyse n'aboutisse, il n'y a
/// donc rien d'autre à interroger. On veut seulement pointer la bonne page d'aide.
fn invoked_command(command: &clap::Command) -> Option<String> {
    let known: Vec<&str> = command
        .get_subcommands()
        .map(|sub| sub.get_name())
        .collect();
    std::env::args()
        .skip(1)
        .find(|argument| known.contains(&argument.as_str()))
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

#[cfg(test)]
mod tests {
    use super::*;

    /// La croix dit déjà que c'est un échec ; « error: » une seconde fois serait du bégaiement.
    #[test]
    fn the_headline_drops_the_prefix_clap_adds() {
        assert_eq!(
            headline("error: unrecognized subcommand 'buidl'\n\n  tip: ..."),
            "unrecognized subcommand 'buidl'"
        );
    }

    /// Un refus dont on ne saurait rien dire reste un refus : la sortie ne doit pas être vide.
    #[test]
    fn an_unreadable_refusal_still_says_something() {
        assert_eq!(headline("   \n\n"), "invalid arguments");
    }
}
