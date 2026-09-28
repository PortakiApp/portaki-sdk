//! `portaki` — command-line toolchain for Portaki Extism Wasm modules.
//!
//! # Role
//!
//! Authors write modules against [`portaki_sdk`]. At build time this binary:
//!
//! 1. Compiles the crate to `wasm32-unknown-unknown`
//! 2. Reads proc-macro JSON under `OUT_DIR/portaki-emissions/`
//! 3. Merges emissions (+ optional hand-written `portaki.module.json`) into `manifest.json`
//! 4. Packages Wasm + manifests, pushed to Portaki's OCI repository by `portaki release`
//!
//! # Commands
//!
//! | Command | Contract |
//! |---------|----------|
//! | `status` | Where the module stands on the developer journey, and the next command |
//! | `doctor` | Check the environment: session, versions, toolchain, cosign, registry, signing identity |
//! | `init` | Scaffold a module crate from a template |
//! | `build` | Produce Wasm + merged manifest (+ migrations/operations bundles + i18n) |
//! | `check` | The gate `release` applies: fmt, clippy, tests, build, manifest, texts |
//! | `add permission\|connector\|language` | Declare what the module uses, where the SDK reads it |
//! | `test` | Forward to `cargo test` in the module crate |
//! | `login` / `logout` | Open or end a developer session (device grant) |
//! | `dev` | Build, deploy to the hosted sandbox, and show what the run did |
//! | `run` | Run a query or command on the sandbox build |
//! | `scenarios` | The seven pathological stays against every surface; `run`, `reset` |
//! | `preview` | Render a surface of the sandbox build |
//! | `reports` | What hosts and the runtime report; `resolve <id> --note` |
//! | `ci` | Answer what a CI workflow used to ask in bash |
//! | `release` | Test, build, push to Portaki's OCI repository, sign, and announce a version |
//! | `release status <v>` / `release notes <v>` | A published version: where it stands, its notes |
//! | `upgrade` | Move the module to another SDK version, and prove nothing broke |
//! | `link` | Open the repository page, or with `--all` link every monorepo module like this one |
//! | `logs` | Follow a module's sandbox logs, optionally one error code |
//! | `catalog` | Dump the SDUI primitive catalog the host understands |
//! | `inspect` | GET a URL and pretty-print it when it is JSON |
//! | `docs` | Print how to open the local SDK documentation |
//!
//! Install: `cargo install portaki-cli`. Requires `rustup target add wasm32-unknown-unknown`.

mod api;
mod auth;
mod changelog;
mod commands;
mod dev_session;
mod exit;
mod http;
mod lang;
mod manifest;
mod oci;
mod oidc;
mod profile;
mod sign;
mod ui;
mod update;
mod watch_lock;
mod workspace;

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

    /// Bare output for scripts and CI: no logo, no headings, no advice. Implies --no-color.
    #[arg(long, global = true)]
    plain: bool,

    /// Print one JSON document on stdout (NDJSON for `logs`) and everything else on stderr.
    #[arg(long, global = true)]
    json: bool,

    /// Base URL of the Portaki API. Defaults to PORTAKI_API_URL, then production.
    #[arg(long, global = true, value_name = "URL", conflicts_with = "env")]
    api: Option<String>,

    /// Named environment: prod, staging, local, or one from ~/.config/portaki/config.toml.
    #[arg(long, global = true, value_name = "NAME")]
    env: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Where the module stands — the five steps, the last version, errors — and what to run next.
    Status(commands::status::StatusArgs),
    /// Check this machine: session, versions, toolchain, cosign, registry, signing identity.
    Doctor(commands::doctor::DoctorArgs),
    /// Scaffold a new module from a template.
    Init(commands::init::InitArgs),
    /// Sign in with the device grant; the session only goes back to the platform that issued it.
    Login(commands::login::LoginArgs),
    /// End the session here and on the platform.
    Logout(commands::login::LogoutArgs),
    /// Build, push to the hosted sandbox, and show what the run did.
    Dev(commands::dev::DevArgs),
    /// Run a query or a command on the sandbox build — bare, list them.
    Run(commands::sandbox::RunArgs),
    /// The seven pathological stays against every surface: show, `run` again, or `reset` fixtures.
    Scenarios(commands::sandbox::ScenariosArgs),
    /// Render a surface of the sandbox build — bare, list them.
    Preview(commands::sandbox::PreviewArgs),
    /// What hosts and the runtime report on a module — and `resolve` once fixed.
    Reports(commands::reports::ReportsArgs),
    /// Build wasm32 artifact, manifest, and i18n bundle.
    Build(commands::build::BuildArgs),
    /// Answer the questions a CI workflow used to ask in bash.
    Ci(commands::ci::CiArgs),
    /// The gate portaki release applies: fmt, clippy, tests, the wasm build, the manifest, the texts.
    Check(commands::check::CheckArgs),
    /// Declare a permission, a built-in connector or a language.
    Add(commands::add::AddArgs),
    /// Move the module to another SDK version, and prove nothing broke.
    Upgrade(commands::sdk::UpgradeArgs),
    /// Show every declared connector, and what it needs to actually call.
    Connectors(commands::connectors::ConnectorsArgs),
    /// Former name of `check --only lint`.
    #[command(hide = true)]
    Lint(commands::lint::LintArgs),
    /// Follow what a module logs in the sandbox, live.
    Logs(commands::logs::LogsArgs),
    /// Run `cargo test` in the module crate.
    Test(commands::test::TestArgs),
    /// Former name of `upgrade`.
    #[command(hide = true)]
    Sdk(commands::sdk::SdkArgs),
    /// Former name of `check --only i18n`.
    #[command(hide = true)]
    I18n(commands::i18n::I18nArgs),
    /// Former name of `add permission`.
    #[command(hide = true)]
    Permissions(commands::permissions::PermissionsArgs),
    /// Test, build, push to Portaki's registry, sign, and announce a version — or `status`/`notes`
    /// of a published one.
    Release(ReleaseCommand),
    /// Former name of `release`.
    #[command(hide = true)]
    Publish(commands::release::ReleaseArgs),
    /// Link this module to its repository — with --all, every module of the monorepo.
    Link(commands::link::LinkArgs),
    /// Print how to open local SDK documentation (no docs server).
    Docs(commands::docs::DocsArgs),
    /// Dump the SDUI catalog specification.
    Catalog,
    /// GET a URL and pretty-print the body when it is JSON (no OCI registry auth).
    Inspect(commands::inspect::InspectArgs),
}

/// `release` publie ; `release status <v>` et `release notes <v>` lisent une version publiée.
#[derive(Debug, clap::Args)]
#[command(args_conflicts_with_subcommands = true)]
struct ReleaseCommand {
    #[command(subcommand)]
    action: Option<commands::release_notes::ReleaseAction>,
    #[command(flatten)]
    args: commands::release::ReleaseArgs,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = parse();
    ui::init(cli.no_color, cli.verbose, cli.plain);
    ui::set_json(cli.json);

    // L'échec est rendu ici, une fois, au lieu du `Debug` que `main() -> Result` imprime : la
    // chaîne des causes se lit, et la sortie d'erreur ressemble au reste de la CLI.
    let ran = match profile::select(cli.api.as_deref(), cli.env.as_deref()) {
        Ok(()) => dispatch(cli.command).await,
        Err(failure) => Err(failure),
    };
    if let Err(failure) = ran {
        ui::report(&failure);
        std::process::exit(exit::code(&failure));
    }

    // Après la commande, jamais avant : l'avis ne retarde rien de ce qu'on attendait, et
    // n'éloigne pas du regard la ligne qu'on est venu lire.
    update::notify().await;
    std::process::exit(exit::success_code());
}

/// Analyse les arguments, en habillant l'aide et `--version` de ce que `clap` ne sait pas seul.
///
/// `clap` rend ces deux écrans pendant l'analyse, donc avant qu'on ait lu le moindre argument :
/// `--no-color` est cherché à la main d'abord, sans quoi un logo en couleurs partirait dans un
/// fichier de sortie qu'on avait justement demandé nu.
fn parse() -> Cli {
    let raw: Vec<String> = std::env::args().collect();
    let bare = raw
        .iter()
        .any(|argument| argument == "--plain" || argument == "--json");
    ui::set_plain(bare);
    ui::set_colors(!bare && !raw.iter().any(|argument| argument == "--no-color"));

    // Le logo et le pied de licence sont ce que `--plain` retire en premier : une aide lue par
    // un script n'a que faire d'une signature de six lignes.
    let mut command = Cli::command().long_version(
        // `clap` veut une chaîne qui vit aussi longtemps que le programme ; celle-ci est
        // construite une fois, au démarrage, et l'écran de version en est le seul lecteur.
        Box::leak(ui::long_version().into_boxed_str()) as &'static str,
    );
    if !bare {
        command = command
            .before_help(ui::banner())
            .before_long_help(ui::banner())
            .after_help(ui::legal())
            .after_long_help(ui::legal());
    }
    let command = command;

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
    if is_a_screen(refusal.kind()) {
        // `clap` rogne l'espace en tête de `before_help` : la ligne qui décolle le logo de
        // l'invite se pose donc ici, sur le flux que `clap` s'apprête à écrire.
        if wants_room() && !ui::plain() {
            if refusal.use_stderr() {
                eprintln!();
            } else {
                println!();
            }
        }
        refusal.exit();
    }

    let rendered = refusal.to_string();
    if std::env::var_os("PORTAKI_RAW_REFUSAL").is_some() {
        eprintln!("RAW>>>\n{rendered}\n<<<END");
    }
    ui::blank();
    let (headline, precisions) = refusal_lines(&rendered);
    ui::failure(headline);
    // « the following required arguments were not provided: » sans la liste qui suit ne dit
    // rien. Le paragraphe entier part, pas sa première ligne.
    for precision in precisions {
        ui::detail(precision);
    }

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
        let arguments: Vec<String> = std::env::args().skip(1).collect();
        let (_, reached) = descend(command, &arguments);
        let commands: Vec<(String, String)> = reached
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
        ui::list(&tr!("commands", "commandes"), &rows);
    }

    let help = match invoked_command(command) {
        Some(name) => format!("portaki {name} --help"),
        None => "portaki --help".to_string(),
    };
    ui::next(&[(
        &help,
        &tr!(
            "every flag this command takes",
            "chaque option de cette commande"
        ),
    )]);
    ui::blank();
    std::process::exit(2);
}

/// `clap` rend-il un écran plutôt qu'un refus ?
///
/// Classé sur le type, et non sur le flux de sortie : `portaki` nu lève
/// `DisplayHelpOnMissingArgumentOrSubcommand`, que `clap` écrit sur stderr. Pris pour un refus,
/// son aide passait dans [`headline`], qui en retenait la première ligne — le logo — et
/// l'affichait derrière une croix.
fn is_a_screen(kind: clap::error::ErrorKind) -> bool {
    use clap::error::ErrorKind;
    matches!(
        kind,
        ErrorKind::DisplayHelp
            | ErrorKind::DisplayVersion
            | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
    )
}

/// Cet écran mérite-t-il qu'on l'aère ?
///
/// Tous sauf `-V` : sa sortie tient en une ligne que des scripts lisent, et une ligne vide
/// devant ferait rendre un vide à `portaki -V | head -1`. `--version` est la forme longue,
/// destinée à un lecteur.
fn wants_room() -> bool {
    room_for(std::env::args().skip(1))
}

/// La décision seule, séparée de l'environnement pour être vérifiable.
fn room_for(mut arguments: impl Iterator<Item = String>) -> bool {
    !arguments.any(|argument| argument == "-V")
}

/// Le refus, coupé en ce qui l'annonce et ce qui le précise.
///
/// `clap` rend un paragraphe, puis une ligne vide, puis l'usage et un renvoi à l'aide — que
/// cette CLI redit elle-même. Ne garder que la première ligne perdait la seule information
/// utile : « the following required arguments were not provided: » ne nomme pas l'argument,
/// ce sont les lignes suivantes qui le font.
///
/// Le « error: » que `clap` préfixe part avec : la croix le dit déjà.
fn refusal_lines(rendered: &str) -> (String, Vec<String>) {
    let mut paragraph = rendered
        .lines()
        .skip_while(|line| line.trim().is_empty())
        .take_while(|line| !line.trim().is_empty())
        .map(|line| line.trim().to_string());

    let headline = paragraph
        .next()
        .map(|line| line.trim_start_matches("error: ").to_string())
        .unwrap_or_else(|| tr!("invalid arguments", "arguments refusés"));
    (headline, paragraph.collect())
}

/// La sous-commande que la ligne de commande nommait, aussi profond qu'elle aille.
///
/// Lue des arguments bruts : le refus est arrivé avant qu'aucune analyse n'aboutisse, il n'y a
/// donc rien d'autre à interroger.
///
/// En descendant l'arbre, et non en s'arrêtant au premier niveau : `portaki ci report`
/// renvoyait à `portaki ci --help`, qui ne dit rien des drapeaux de `report` — la page
/// manquée était justement celle qu'on venait chercher.
fn invoked_command(command: &clap::Command) -> Option<String> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let (path, _) = descend(command, &arguments);
    (!path.is_empty()).then(|| path.join(" "))
}

/// Le chemin parcouru et la commande atteinte.
///
/// Séparé des arguments du processus pour être vérifiable, et rendant les deux parce que les
/// deux servent : le chemin nomme la page d'aide, la commande atteinte porte les
/// sous-commandes à proposer. `portaki ci nope` déroulait les commandes racines, qui ne
/// répondent pas à la question posée.
fn descend<'a>(root: &'a clap::Command, arguments: &[String]) -> (Vec<String>, &'a clap::Command) {
    let mut node = root;
    let mut path = Vec::new();
    for argument in arguments {
        // Un argument qui n'est pas une sous-commande n'interrompt pas la descente : les
        // drapeaux globaux peuvent précéder la commande.
        if let Some(next) = node
            .get_subcommands()
            .find(|sub| sub.get_name() == argument.as_str())
        {
            path.push(next.get_name().to_string());
            node = next;
        }
    }
    (path, node)
}

async fn dispatch(command: Command) -> Result<()> {
    match command {
        Command::Status(args) => commands::status::run(args).await,
        Command::Doctor(args) => commands::doctor::run(args).await,
        Command::Init(args) => commands::init::run(args),
        Command::Login(args) => commands::login::run(args).await,
        Command::Logout(args) => commands::login::logout(args).await,
        Command::Dev(args) => commands::dev::run(args).await,
        Command::Run(args) => commands::sandbox::run(args).await,
        Command::Scenarios(args) => commands::sandbox::scenarios(args).await,
        Command::Preview(args) => commands::sandbox::preview(args).await,
        Command::Reports(args) => commands::reports::run(args).await,
        Command::Build(args) => commands::build::run(args).await,
        Command::Ci(args) => commands::ci::run(args).await,
        Command::Check(args) => commands::check::run(args).await,
        Command::Add(args) => commands::add::run(args),
        Command::Upgrade(args) => commands::sdk::upgrade(args).await,
        Command::Connectors(args) => commands::connectors::run(args),
        Command::Lint(args) => commands::lint::run(args),
        Command::Logs(args) => commands::logs::run(args).await,
        Command::Test(args) => commands::test::run(args),
        Command::Sdk(args) => commands::sdk::run(args).await,
        Command::Permissions(args) => commands::permissions::run(args),
        Command::I18n(args) => commands::i18n::run(args),
        Command::Release(ReleaseCommand {
            action: Some(action),
            ..
        }) => commands::release_notes::run(action).await,
        Command::Release(ReleaseCommand { args, .. }) => commands::release::run(args).await,
        Command::Publish(args) => {
            ui::warn("portaki publish is now portaki release — use that name from now on");
            commands::release::run(args).await
        }
        Command::Link(args) => commands::link::run(args).await,
        Command::Docs(args) => commands::docs::run(args),
        Command::Catalog => commands::catalog::run(),
        Command::Inspect(args) => commands::inspect::run(args).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La croix dit déjà que c'est un échec ; « error: » une seconde fois serait du bégaiement.
    #[test]
    fn the_headline_drops_the_prefix_clap_adds() {
        let (headline, _) = refusal_lines("error: unrecognized subcommand 'buidl'\n\n  tip: ...");

        assert_eq!(headline, "unrecognized subcommand 'buidl'");
    }

    /// « the following required arguments were not provided: » ne nomme pas l'argument : ce
    /// sont les lignes suivantes qui le font, et les perdre laissait un refus qui ne dit rien.
    #[test]
    fn the_precisions_that_name_the_argument_survive() {
        let rendered = concat!(
            "error: the following required arguments were not provided:\n",
            "  --outcome <OUTCOME>\n",
            "\n",
            "Usage: portaki ci report --outcome <OUTCOME>\n",
            "\n",
            "For more information, try '--help'.\n"
        );

        let (headline, precisions) = refusal_lines(rendered);

        assert_eq!(
            headline,
            "the following required arguments were not provided:"
        );
        assert_eq!(precisions, vec!["--outcome <OUTCOME>"]);
    }

    /// L'usage et le renvoi à l'aide sont après la ligne vide : cette CLI les redit elle-même,
    /// les recopier ferait doublon.
    #[test]
    fn what_follows_the_blank_line_is_left_to_clap() {
        let (_, precisions) = refusal_lines("error: nope\n\nUsage: portaki\n");

        assert!(precisions.is_empty());
    }

    /// `portaki ci report` renvoyait à `portaki ci --help`, qui ne dit rien des drapeaux de
    /// `report` — la page manquée était justement celle qu'on venait chercher.
    #[test]
    fn the_help_pointer_reaches_the_deepest_command() {
        let root = Cli::command();
        let args = |raw: &[&str]| raw.iter().map(ToString::to_string).collect::<Vec<_>>();

        let (path, reached) = descend(&root, &args(&["ci", "report"]));
        assert_eq!(path, vec!["ci", "report"]);
        assert_eq!(reached.get_name(), "report");

        // Les drapeaux globaux peuvent précéder la commande sans interrompre la descente.
        let (path, _) = descend(&root, &args(&["--plain", "ci", "modules"]));
        assert_eq!(path, vec!["ci", "modules"]);

        // Une sous-commande inconnue laisse le noeud atteint sur son parent : ce sont ses
        // sous-commandes qu'il faut proposer, pas celles de la racine.
        let (path, reached) = descend(&root, &args(&["ci", "nope"]));
        assert_eq!(path, vec!["ci"]);
        assert_eq!(reached.get_name(), "ci");

        let (path, reached) = descend(&root, &args(&["buidl"]));
        assert!(path.is_empty());
        assert_eq!(reached.get_name(), "portaki");
    }

    fn args(raw: &[&str]) -> impl Iterator<Item = String> + use<> {
        raw.iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .into_iter()
    }

    /// `-V` tient en une ligne que des scripts lisent : une ligne vide devant la rendrait vide.
    #[test]
    fn the_short_version_stays_a_single_parseable_line() {
        assert!(!room_for(args(&["-V"])));
        assert!(room_for(args(&["--version"])));
        assert!(room_for(args(&["--help"])));
        assert!(room_for(args(&[])));
    }

    /// `portaki` nu doit ouvrir l'aide, pas une croix suivie du logo.
    #[test]
    fn a_help_screen_is_never_taken_for_a_refusal() {
        use clap::error::ErrorKind;

        assert!(is_a_screen(
            ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
        ));
        assert!(is_a_screen(ErrorKind::DisplayHelp));
        assert!(is_a_screen(ErrorKind::DisplayVersion));
        assert!(!is_a_screen(ErrorKind::InvalidSubcommand));
        assert!(!is_a_screen(ErrorKind::UnknownArgument));
    }

    /// L'aide dit ce que la commande fait : pas de Scaleway, pas de passerelle locale.
    #[test]
    fn the_help_describes_what_the_commands_do() {
        let root = Cli::command();
        let about = |name: &str| {
            root.find_subcommand(name)
                .and_then(|sub| sub.get_about().map(ToString::to_string))
                .unwrap_or_default()
        };

        assert!(!about("release").contains("Scaleway"));
        assert!(about("release").contains("registry"));
        assert!(about("dev").contains("hosted sandbox"));
        assert!(!about("inspect").contains("OCI artifact"));
    }

    /// Deux arguments du même nom, un groupe en double : `clap` ne le dit qu'à l'exécution.
    #[test]
    fn the_command_tree_is_valid() {
        Cli::command().debug_assert();
    }

    /// Un refus dont on ne saurait rien dire reste un refus : la sortie ne doit pas être vide.
    #[test]
    fn an_unreadable_refusal_still_says_something() {
        let (headline, precisions) = refusal_lines("   \n\n");

        assert_eq!(headline, "invalid arguments");
        assert!(precisions.is_empty());
    }
}
