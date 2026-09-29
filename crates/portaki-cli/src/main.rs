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

/// `clap`'s help painted like the rest of the output: a single visual vocabulary, whether a
/// line comes from `--help` or from a command.
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

/// `release` publishes; `release status <v>` and `release notes <v>` read a published version.
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

    // The failure is rendered here, once, instead of the `Debug` that `main() -> Result` prints:
    // the chain of causes reads, and the error output looks like the rest of the CLI.
    let ran = match profile::select(cli.api.as_deref(), cli.env.as_deref()) {
        Ok(()) => dispatch(cli.command).await,
        Err(failure) => Err(failure),
    };
    if let Err(failure) = ran {
        ui::report(&failure);
        std::process::exit(exit::code(&failure));
    }

    // After the command, never before: the notice delays nothing of what was being waited for,
    // and does not push the line one came to read out of sight.
    update::notify().await;
    std::process::exit(exit::success_code());
}

/// Parses the arguments, dressing the help and `--version` in what `clap` does not know on its
/// own.
///
/// `clap` renders those two screens during parsing, hence before a single argument has been read:
/// `--no-color` is looked for by hand first, without which a coloured logo would land in an
/// output file that had been asked for bare precisely to avoid that.
fn parse() -> Cli {
    let raw: Vec<String> = std::env::args().collect();
    let bare = raw
        .iter()
        .any(|argument| argument == "--plain" || argument == "--json");
    ui::set_plain(bare);
    ui::set_colors(!bare && !raw.iter().any(|argument| argument == "--no-color"));

    // The logo and the licence footer are the first things `--plain` takes away: help read by a
    // script has no use for a six-line signature.
    let mut command = Cli::command().long_version(
        // `clap` wants a string that lives as long as the program; this one is built once, at
        // startup, and the version screen is its only reader.
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

/// Renders `clap`'s refusal like the rest of the CLI, and says where to look.
///
/// "a value is required for '--dispatch <DISPATCH>'" is exact and does not help: what one could
/// have written is missing from it. When the refusal is about the command itself, the list of
/// commands follows — it is the only answer to "which one?".
fn refuse(refusal: clap::Error, command: &clap::Command) -> ! {
    use clap::error::ErrorKind;

    // `--help` and `--version` are not failures: `clap` renders them itself and exits with 0.
    if is_a_screen(refusal.kind()) {
        // `clap` trims the whitespace at the head of `before_help`: the line that lifts the
        // logo off the prompt is therefore put here, on the stream `clap` is about to write to.
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
    // "the following required arguments were not provided:" without the list that follows says
    // nothing. The whole paragraph goes out, not just its first line.
    for precision in precisions {
        ui::detail(precision);
    }

    // `clap` can often suggest the name that was aimed at; losing it would mean taking the one
    // really useful thing out of its message.
    for tip in rendered
        .lines()
        .filter_map(|line| line.trim().strip_prefix("tip: "))
    {
        ui::detail(tip);
    }

    // The list of commands only answers "which one?". On an unknown flag, one is already inside
    // a command: unrolling the whole of it would be noise in front of the real question.
    if matches!(
        refusal.kind(),
        ErrorKind::InvalidSubcommand | ErrorKind::MissingSubcommand
    ) {
        // `get_about` returns a `StyledStr`: it has to be materialised before slices of it can
        // be lent to the list.
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

/// Is `clap` rendering a screen rather than a refusal?
///
/// Classified on the kind, and not on the output stream: bare `portaki` raises
/// `DisplayHelpOnMissingArgumentOrSubcommand`, which `clap` writes to stderr. Taken for a
/// refusal, its help went through [`headline`], which kept its first line — the logo — and
/// displayed it behind a cross.
fn is_a_screen(kind: clap::error::ErrorKind) -> bool {
    use clap::error::ErrorKind;
    matches!(
        kind,
        ErrorKind::DisplayHelp
            | ErrorKind::DisplayVersion
            | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
    )
}

/// Does this screen deserve some air?
///
/// All of them except `-V`: its output fits on one line that scripts read, and a blank line in
/// front of it would make `portaki -V | head -1` return an empty one. `--version` is the long
/// form, meant for a reader.
fn wants_room() -> bool {
    room_for(std::env::args().skip(1))
}

/// The decision alone, separated from the environment so that it can be checked.
fn room_for(mut arguments: impl Iterator<Item = String>) -> bool {
    !arguments.any(|argument| argument == "-V")
}

/// The refusal, cut into what announces it and what spells it out.
///
/// `clap` renders a paragraph, then a blank line, then the usage and a pointer to the help —
/// which this CLI says again itself. Keeping only the first line lost the one useful piece of
/// information: "the following required arguments were not provided:" does not name the
/// argument, it is the lines that follow that do.
///
/// The "error: " that `clap` prefixes goes with it: the cross says so already.
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

/// The subcommand the command line named, however deep it goes.
///
/// Read off the raw arguments: the refusal came before any parsing went through, so there is
/// nothing else to ask.
///
/// By walking down the tree, and not by stopping at the first level: `portaki ci report` used to
/// point at `portaki ci --help`, which says nothing about `report`'s flags — the page that was
/// missed was precisely the one being looked for.
fn invoked_command(command: &clap::Command) -> Option<String> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let (path, _) = descend(command, &arguments);
    (!path.is_empty()).then(|| path.join(" "))
}

/// The path walked and the command reached.
///
/// Separated from the process's arguments so that it can be checked, and returning both because
/// both are used: the path names the help page, the command reached carries the subcommands to
/// suggest. `portaki ci nope` used to unroll the root commands, which do not answer the question
/// being asked.
fn descend<'a>(root: &'a clap::Command, arguments: &[String]) -> (Vec<String>, &'a clap::Command) {
    let mut node = root;
    let mut path = Vec::new();
    for argument in arguments {
        // An argument that is not a subcommand does not interrupt the descent: global flags may
        // come before the command.
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

    /// The cross already says it is a failure; "error: " a second time would be a stutter.
    #[test]
    fn the_headline_drops_the_prefix_clap_adds() {
        let (headline, _) = refusal_lines("error: unrecognized subcommand 'buidl'\n\n  tip: ...");

        assert_eq!(headline, "unrecognized subcommand 'buidl'");
    }

    /// "the following required arguments were not provided:" does not name the argument: it is
    /// the lines that follow that do, and losing them left a refusal that says nothing.
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

    /// The usage and the pointer to the help come after the blank line: this CLI says them
    /// again itself, so copying them out would duplicate them.
    #[test]
    fn what_follows_the_blank_line_is_left_to_clap() {
        let (_, precisions) = refusal_lines("error: nope\n\nUsage: portaki\n");

        assert!(precisions.is_empty());
    }

    /// `portaki ci report` used to point at `portaki ci --help`, which says nothing about
    /// `report`'s flags — the page that was missed was precisely the one being looked for.
    #[test]
    fn the_help_pointer_reaches_the_deepest_command() {
        let root = Cli::command();
        let args = |raw: &[&str]| raw.iter().map(ToString::to_string).collect::<Vec<_>>();

        let (path, reached) = descend(&root, &args(&["ci", "report"]));
        assert_eq!(path, vec!["ci", "report"]);
        assert_eq!(reached.get_name(), "report");

        // Global flags may come before the command without interrupting the descent.
        let (path, _) = descend(&root, &args(&["--plain", "ci", "modules"]));
        assert_eq!(path, vec!["ci", "modules"]);

        // An unknown subcommand leaves the node reached on its parent: it is the parent's
        // subcommands that should be suggested, not the root's.
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

    /// `-V` fits on one line that scripts read: a blank line in front would make it come back
    /// empty.
    #[test]
    fn the_short_version_stays_a_single_parseable_line() {
        assert!(!room_for(args(&["-V"])));
        assert!(room_for(args(&["--version"])));
        assert!(room_for(args(&["--help"])));
        assert!(room_for(args(&[])));
    }

    /// Bare `portaki` must open the help, not a cross followed by the logo.
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

    /// The help says what the command does: no Scaleway, no local gateway.
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

    /// Two arguments with the same name, a duplicated group: `clap` only says so at runtime.
    #[test]
    fn the_command_tree_is_valid() {
        Cli::command().debug_assert();
    }

    /// A refusal nothing could be said about is still a refusal: the output must not be empty.
    #[test]
    fn an_unreadable_refusal_still_says_something() {
        let (headline, precisions) = refusal_lines("   \n\n");

        assert_eq!(headline, "invalid arguments");
        assert!(precisions.is_empty());
    }
}
