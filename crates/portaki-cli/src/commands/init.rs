//! `portaki init` — scaffold a module from templates.

use std::fs;
use std::io::{BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::{Parser, ValueEnum};
use include_dir::{include_dir, Dir};

use crate::ui;

/// The scaffolding, compiled into the binary.
///
/// Read from disk, it resolved against this crate's source directory — a path that exists in a
/// checkout of this repository and nowhere else, so `cargo install portaki-cli` produced a
/// command that could not scaffold anything.
static TEMPLATES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates");

#[derive(Debug, Clone, ValueEnum)]
/// Template kind for `portaki init`.
pub enum InitTemplate {
    /// Default module with entity, surfaces, and i18n bundles.
    Default,
    /// Minimal empty module skeleton.
    Empty,
}

#[derive(Debug, Parser)]
/// Arguments for `portaki init`.
pub struct InitArgs {
    /// Module name (kebab-case recommended).
    pub name: String,
    /// Template to use.
    #[arg(long, value_enum, default_value_t = InitTemplate::Default)]
    pub template: InitTemplate,
    /// Output directory (defaults to `./{name}`).
    #[arg(long)]
    pub path: Option<PathBuf>,
    /// Ask nothing: the template as is, plus whatever the options below set.
    #[arg(long, short = 'y')]
    pub yes: bool,
    /// Display name, in `i18n/*.json` (`module.displayName`).
    #[arg(long)]
    pub display_name: Option<String>,
    /// One-sentence description, in `i18n/fr-FR.json` and `listing.json` (French).
    #[arg(long)]
    pub description: Option<String>,
    /// Catalogue tagline, in `listing.json` (French, 90 characters at most).
    #[arg(long)]
    pub tagline: Option<String>,
    /// Catalogue category, in `listing.json`.
    #[arg(long, value_enum)]
    pub category: Option<Category>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
/// The moment of the stay a module belongs to — the closed list of the registry.
pub enum Category {
    /// Getting in, check-in.
    Arrival,
    /// Life in the rental.
    Stay,
    /// The neighbourhood, outings.
    Around,
    /// Forms, rules, paperwork.
    Formalities,
}

impl Category {
    const ALL: [Category; 4] = [
        Category::Arrival,
        Category::Stay,
        Category::Around,
        Category::Formalities,
    ];

    fn wire(self) -> &'static str {
        match self {
            Category::Arrival => "arrival",
            Category::Stay => "stay",
            Category::Around => "around",
            Category::Formalities => "formalities",
        }
    }
}

/// The registry refuses a longer tagline: past it, the catalogue card truncates.
const TAGLINE_MAX: usize = 90;

/// What the author chose — `None` keeps the template's text.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct Answers {
    display_name: Option<String>,
    description: Option<String>,
    tagline: Option<String>,
    category: Option<Category>,
    author: Option<String>,
}

impl Answers {
    /// The JSON-string placeholders of the templates, each with its value or the template default.
    ///
    /// The defaults of `listing.json` start with « À compléter » on purpose: the `listing`
    /// conformance check recognises them and refuses to publish them.
    fn placeholders(&self, module_name: &str) -> [(&'static str, String); 6] {
        let or = |value: &Option<String>, default: &str| {
            value.clone().unwrap_or_else(|| default.to_string())
        };
        [
            ("{{DISPLAY_NAME}}", or(&self.display_name, module_name)),
            (
                "{{DESCRIPTION}}",
                or(&self.description, "Ce que ce module fait, en une phrase."),
            ),
            (
                "{{LISTING_DESCRIPTION}}",
                or(
                    &self.description,
                    "À compléter : ce que le module fait pour l'hôte et le voyageur, en quelques phrases.",
                ),
            ),
            (
                "{{TAGLINE}}",
                or(
                    &self.tagline,
                    "À compléter : ce que le module apporte, en une phrase.",
                ),
            ),
            (
                "{{CATEGORY}}",
                self.category.unwrap_or(Category::Stay).wire().to_string(),
            ),
            ("{{AUTHOR_NAME}}", or(&self.author, "TODO")),
        ]
    }
}

/// `value` as it goes between the quotes of a JSON string — quotes and backslashes escaped.
fn json_escaped(value: &str) -> String {
    let quoted = serde_json::to_string(value).expect("a string serialises");
    quoted[1..quoted.len() - 1].to_string()
}

fn too_long(tagline: &str) -> bool {
    tagline.chars().count() > TAGLINE_MAX
}

/// Asks for what the options did not set; Enter skips a question and keeps the default.
///
/// Reads from `input` rather than stdin so that a test can answer. End of input answers
/// everything left with Enter.
fn ask(
    input: &mut impl BufRead,
    output: &mut impl Write,
    module_name: &str,
    git_author: Option<String>,
    mut answers: Answers,
) -> Result<Answers> {
    writeln!(output, "    Enter keeps the default.")?;
    if answers.display_name.is_none() {
        answers.display_name = prompt(input, output, "display name", module_name)?;
    }
    if answers.description.is_none() {
        answers.description = prompt(input, output, "description, in one sentence", "")?;
    }
    if answers.tagline.is_none() {
        answers.tagline = loop {
            match prompt(
                input,
                output,
                "catalogue tagline, 90 characters at most",
                "",
            )? {
                Some(tagline) if too_long(&tagline) => writeln!(
                    output,
                    "    {} characters — {TAGLINE_MAX} at most, try shorter.",
                    tagline.chars().count()
                )?,
                tagline => break tagline,
            }
        };
    }
    if answers.category.is_none() {
        for (index, category) in Category::ALL.iter().enumerate() {
            writeln!(output, "    {}. {}", index + 1, category.wire())?;
        }
        answers.category = loop {
            let Some(choice) = prompt(input, output, "category", "stay")? else {
                break None;
            };
            let found = Category::ALL.iter().enumerate().find(|(index, category)| {
                choice == (index + 1).to_string() || choice.eq_ignore_ascii_case(category.wire())
            });
            match found {
                Some((_, category)) => break Some(*category),
                None => writeln!(output, "    no category {choice:?} — 1 to 4, or its name.")?,
            }
        };
    }
    let author_default = git_author.clone().unwrap_or_else(|| "TODO".to_string());
    answers.author = prompt(input, output, "author name", &author_default)?.or(git_author);
    Ok(answers)
}

/// One question. `None` when the answer is empty — the template keeps its text.
fn prompt(
    input: &mut impl BufRead,
    output: &mut impl Write,
    question: &str,
    default: &str,
) -> Result<Option<String>> {
    if default.is_empty() {
        write!(output, "    {question}: ")?;
    } else {
        write!(output, "    {question} [{default}]: ")?;
    }
    output.flush()?;
    let mut answer = String::new();
    input.read_line(&mut answer)?;
    let answer = answer.trim();
    Ok((!answer.is_empty()).then(|| answer.to_string()))
}

/// `git config user.name`, when git is there and has one.
fn git_user_name() -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["config", "user.name"])
        .output()
        .ok()?;
    let name = String::from_utf8(output.stdout).ok()?.trim().to_string();
    (output.status.success() && !name.is_empty()).then_some(name)
}

/// Runs `portaki init`.
pub fn run(args: InitArgs) -> Result<()> {
    ui::header(
        "portaki init",
        "Scaffold a module crate — buildable, runnable in the sandbox, publishable.",
    );

    let dest = args
        .path
        .clone()
        .unwrap_or_else(|| PathBuf::from(&args.name));

    let template_dir = TEMPLATES
        .get_dir(directory(&args.template))
        .with_context(|| {
            format!(
                "template missing from this build: {}",
                label(&args.template)
            )
        })?;

    if dest.exists() && !dest.is_dir() {
        bail!("destination is not a directory: {}", dest.display());
    }

    // A cloned repository is the usual starting point — the directory is there, and holds a
    // `.git` and maybe a licence. Only a file the scaffold would overwrite is a reason to stop.
    let clashes = clashes(&dest, &planned_paths(template_dir));
    if !clashes.is_empty() {
        bail!(
            "{} already has {} — move them aside, or scaffold elsewhere",
            dest.display(),
            listed(&clashes)
        );
    }

    if args.tagline.as_deref().is_some_and(too_long) {
        bail!("--tagline is longer than {TAGLINE_MAX} characters — the registry refuses it");
    }
    let preset = Answers {
        display_name: args.display_name.clone(),
        description: args.description.clone(),
        tagline: args.tagline.clone(),
        category: args.category,
        author: None,
    };
    // Outside a terminal (a CI, a pipe) a question would wait forever: the template as is.
    let answers = if args.yes || !std::io::stdin().is_terminal() {
        preset
    } else {
        ask(
            &mut std::io::stdin().lock(),
            &mut std::io::stdout(),
            &args.name,
            git_user_name(),
            preset,
        )?
    };

    let scaffolding = ui::step(format!(
        "scaffolding {} from the {} template",
        args.name,
        label(&args.template)
    ));
    copy_template(template_dir, &dest, &args.name, &answers)?;
    scaffolding.done(format!("created {}", dest.display()));

    describe(&args.template);
    let mut next: Vec<(&str, &str)> = Vec::new();
    let cd = format!("cd {}", dest.display());
    // Scaffolded in place — `cd .` would be a step that does nothing.
    if dest != Path::new(".") {
        next.push((cd.as_str(), "everything below runs from the module root"));
    }
    next.push((
        "portaki build",
        "compile to wasm32 and assemble the manifest",
    ));
    next.push((
        "portaki dev --watch",
        "run it in the hosted sandbox on every save",
    ));
    ui::next(&next);
    ui::blank();
    Ok(())
}

/// Ce qui vient d'être écrit, et à quoi chaque morceau sert.
///
/// Un squelette qu'on découvre fichier par fichier se lit mal : `ids.rs` et `i18n/` n'ont de
/// sens que l'un par rapport à l'autre, et rien dans leur nom ne le dit.
fn describe(template: &InitTemplate) {
    let mut rows = vec![
        ("src/lib.rs", "the module — entity, capability, manifest"),
        ("src/ids.rs", "typed surface and operation ids"),
    ];
    if matches!(template, InitTemplate::Default) {
        rows.push(("src/host/", "surfaces the host dashboard renders"));
        rows.push(("src/guest/", "surfaces the guest booklet renders"));
        rows.push((
            "src/commands.rs",
            "updateConfig — what the sheet's Save posts",
        ));
        rows.push((
            "src/queries.rs",
            "getConfig — what the dashboard reads back",
        ));
        rows.push(("src/config.rs", "the settings blob, in the module's own KV"));
        rows.push(("tests/", "the mock host, the settings round-tripped"));
        rows.push((
            "tests/conformance.rs",
            "the battery every module passes before it publishes",
        ));
    }
    rows.push((
        "i18n/*.json",
        "one file per locale — the keys ids.rs points at",
    ));
    rows.push(("Cargo.toml", "wired to portaki-sdk, cdylib for wasm32"));
    rows.push((
        "build.rs",
        "no build step — it exists so cargo gives the macros an OUT_DIR",
    ));
    rows.push((
        "src/lib.rs",
        "portaki_module! — author, icon, maturity; build writes the catalogue from the code",
    ));
    rows.push((
        "listing.json",
        "the public listing — published with each release",
    ));

    ui::list("what you got", &rows);
}

fn label(template: &InitTemplate) -> &'static str {
    match template {
        InitTemplate::Default => "default",
        InitTemplate::Empty => "empty",
    }
}

/// Names a few of the clashing paths and counts the rest.
///
/// Scaffolding over an existing module clashes on every file; seventeen paths on one line say
/// less than three and a number.
fn listed(paths: &[PathBuf]) -> String {
    const SHOWN: usize = 3;
    let named = paths
        .iter()
        .take(SHOWN)
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    match paths.len().saturating_sub(SHOWN) {
        0 => named,
        rest => format!("{named} and {rest} more"),
    }
}

/// Every path this scaffold would write, relative to the destination.
fn planned_paths(source: &Dir<'_>) -> Vec<PathBuf> {
    let root = source.path();
    let mut planned = Vec::new();
    collect_paths(source, root, &mut planned);
    planned
}

fn collect_paths(source: &Dir<'_>, root: &Path, planned: &mut Vec<PathBuf>) {
    for file in source.files() {
        let relative = file.path().strip_prefix(root).unwrap_or(file.path());
        planned.push(rendered_path(relative));
    }
    for child in source.dirs() {
        collect_paths(child, root, planned);
    }
}

/// The same `.template` strip `copy_template` applies, on a whole path.
fn rendered_path(relative: &Path) -> PathBuf {
    let Some(name) = relative
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
    else {
        return relative.to_path_buf();
    };
    let stripped = name.strip_suffix(".template").unwrap_or(&name);
    relative.with_file_name(stripped)
}

/// Which of those already exist — the only reason to refuse a directory that is already there.
fn clashes(dest: &Path, planned: &[PathBuf]) -> Vec<PathBuf> {
    planned
        .iter()
        .filter(|path| dest.join(path).exists())
        .cloned()
        .collect()
}

/// What `use` statements have to spell: cargo turns a kebab-case package into a snake_case lib.
fn crate_name(module_name: &str) -> String {
    module_name.replace('-', "_")
}

fn directory(template: &InitTemplate) -> &'static str {
    match template {
        InitTemplate::Default => "default-module",
        InitTemplate::Empty => "empty-module",
    }
}

/// Writes an embedded directory out, rendering each file on the way.
fn copy_template(
    source: &Dir<'_>,
    dest: &Path,
    module_name: &str,
    answers: &Answers,
) -> Result<()> {
    fs::create_dir_all(dest).with_context(|| format!("create {}", dest.display()))?;

    for file in source.files() {
        let name = file
            .path()
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default();
        // `Cargo.toml.template` would otherwise make the scaffolded crate a cargo package the
        // moment it is written, and cargo would read it while it still holds placeholders.
        let name = name.strip_suffix(".template").unwrap_or(&name).to_string();
        let target = dest.join(&name);

        let text = file
            .contents_utf8()
            .with_context(|| format!("template {} is not UTF-8", file.path().display()))?;
        // The CLI's version is the SDK it was published with: a scaffolded module compiles
        // against the SDK this command knows, not against whatever is newest.
        let mut rendered = text.to_string();
        // Inside JSON strings only, so escaped as JSON; before `{{MODULE_NAME}}`, which is a
        // default of one of them.
        for (placeholder, value) in answers.placeholders(module_name) {
            rendered = rendered.replace(placeholder, &json_escaped(&value));
        }
        let rendered = rendered
            .replace("{{MODULE_NAME}}", module_name)
            .replace("{{CRATE_NAME}}", &crate_name(module_name))
            .replace("{{SDK_VERSION}}", env!("CARGO_PKG_VERSION"));
        fs::write(&target, rendered).with_context(|| format!("write {}", target.display()))?;
    }

    for child in source.dirs() {
        let name = child
            .path()
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default();
        copy_template(child, &dest.join(name), module_name, answers)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Both templates have to be in the binary, or `init` only fails for whoever installed it.
    #[test]
    fn every_template_is_embedded() {
        for template in [InitTemplate::Default, InitTemplate::Empty] {
            let dir = TEMPLATES
                .get_dir(directory(&template))
                .expect("template embedded");
            assert!(dir.files().count() + dir.dirs().count() > 0);
        }
    }

    #[test]
    fn what_a_scaffold_would_write_is_known_before_it_writes() {
        let planned = planned_paths(TEMPLATES.get_dir("default-module").expect("template"));

        // Rendered names, not template ones — that is what a clash has to be checked against.
        assert!(planned.contains(&PathBuf::from("Cargo.toml")));
        // No manifest to write: `portaki build` derives it from the code.
        assert!(!planned.contains(&PathBuf::from("portaki.module.json")));
        assert!(planned.contains(&PathBuf::from("src/host/mod.rs")));
        assert!(planned.contains(&PathBuf::from(".cargo/config.toml")));
        assert!(!planned
            .iter()
            .any(|path| path.to_string_lossy().ends_with(".template")));
    }

    #[test]
    fn a_long_clash_is_three_names_and_a_count() {
        let paths: Vec<PathBuf> = ["Cargo.toml", "build.rs", "src/lib.rs", "i18n/en-US.json"]
            .iter()
            .map(PathBuf::from)
            .collect();

        assert_eq!(listed(&paths[..2]), "Cargo.toml, build.rs");
        assert_eq!(
            listed(&paths),
            "Cargo.toml, build.rs, src/lib.rs and 1 more"
        );
    }

    /// The point of #115: a cloned repository is a directory that already exists.
    #[test]
    fn an_existing_directory_is_fine_until_a_file_would_be_overwritten() {
        let dest = std::env::temp_dir().join(format!("portaki-clash-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dest);
        fs::create_dir_all(dest.join(".git")).expect("a clone");
        let planned = planned_paths(TEMPLATES.get_dir("default-module").expect("template"));

        assert!(clashes(&dest, &planned).is_empty());

        fs::write(dest.join("Cargo.toml"), "[package]").expect("an existing crate");
        assert_eq!(clashes(&dest, &planned), vec![PathBuf::from("Cargo.toml")]);

        fs::remove_dir_all(&dest).ok();
    }

    #[test]
    fn a_kebab_case_module_becomes_a_snake_case_crate() {
        assert_eq!(crate_name("pre-arrival-form"), "pre_arrival_form");
        assert_eq!(crate_name("trmnl"), "trmnl");
    }

    /// What the two commands `init` recommends need in order to run at all.
    #[test]
    fn a_scaffold_has_what_build_and_dev_read() {
        for template in [InitTemplate::Default, InitTemplate::Empty] {
            let dir = TEMPLATES.get_dir(directory(&template)).expect("template");
            let names: Vec<String> = dir
                .files()
                .map(|file| {
                    file.path()
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .to_string()
                })
                .collect();

            // `portaki build` reads emissions from OUT_DIR, which only a build script creates.
            assert!(names.iter().any(|name| name == "build.rs"), "{names:?}");
            // Nothing hand-written for the catalogue: the code declares it.
            assert!(
                !names
                    .iter()
                    .any(|name| name.starts_with("portaki.module.json")),
                "{names:?}"
            );
            // Without the custom getrandom backend, the wasm32 build stops inside getrandom.
            let cargo_config = dir
                .get_file(format!(
                    "{}/.cargo/config.toml.template",
                    directory(&template)
                ))
                .expect("wasm rustflags");
            assert!(cargo_config
                .contents_utf8()
                .unwrap_or_default()
                .contains("getrandom_backend"));
        }
    }

    #[test]
    fn a_scaffolded_module_carries_its_name_and_the_sdk_version() {
        let dest = std::env::temp_dir().join(format!("portaki-init-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dest);

        copy_template(
            TEMPLATES.get_dir("default-module").expect("template"),
            &dest,
            "concierge",
            &Answers::default(),
        )
        .expect("scaffold");

        let cargo = fs::read_to_string(dest.join("Cargo.toml")).expect("Cargo.toml written");
        assert!(cargo.contains("name = \"concierge\""));
        assert!(cargo.contains(env!("CARGO_PKG_VERSION")));
        assert!(!cargo.contains("{{"));
        // Nested and dot directories come out too — the wasm rustflags live in one of them.
        assert!(dest.join("src/host/mod.rs").exists());
        assert!(dest.join(".cargo/config.toml").exists());
        assert!(!dest.join("Cargo.toml.template").exists());
        // A crate name is not a module id: `use` statements need the snake_case spelling.
        let integration =
            fs::read_to_string(dest.join("tests/integration.rs")).expect("tests written");
        assert!(integration.contains("use concierge::{"));
        assert!(!integration.contains("{{"));
        // Every new module runs the conformance battery `portaki publish` gates on.
        let conformance =
            fs::read_to_string(dest.join("tests/conformance.rs")).expect("battery written");
        assert!(conformance.contains("portaki_test_utils::conformance!();"));
        let lib = fs::read_to_string(dest.join("src/lib.rs")).expect("lib written");
        assert!(lib.contains("id = \"concierge\""));
        assert!(!dest.join("portaki.module.json").exists());

        fs::remove_dir_all(&dest).ok();
    }

    fn scaffold(template: &str, answers: &Answers) -> tempfile::TempDir {
        let dest = tempfile::tempdir().expect("tempdir");
        copy_template(
            TEMPLATES.get_dir(template).expect("template"),
            dest.path(),
            "concierge",
            answers,
        )
        .expect("scaffold");
        dest
    }

    fn json(path: &Path) -> serde_json::Value {
        serde_json::from_str(&fs::read_to_string(path).expect("written")).expect("JSON")
    }

    /// `init --yes`: a listing the registry accepts, only waiting for its texts.
    #[test]
    fn the_scaffolded_listing_follows_the_schema() {
        let schema: serde_json::Value =
            serde_json::from_str(portaki_test_utils::conformance::LISTING_SCHEMA_V1).unwrap();
        let validator = jsonschema::validator_for(&schema).expect("schema compiles");
        for (template, guest) in [("default-module", true), ("empty-module", false)] {
            let dest = scaffold(template, &Answers::default());
            let listing = json(&dest.path().join("listing.json"));

            let errors: Vec<String> = validator
                .iter_errors(&listing)
                .map(|e| e.to_string())
                .collect();
            assert!(errors.is_empty(), "{template}: {errors:?}");
            assert_eq!(listing["category"], "stay");
            assert_eq!(listing["guestSurface"].is_object(), guest, "{template}");
            // The instructions the `listing` conformance check refuses to publish.
            let tagline = listing["tagline"]["fr"].as_str().unwrap();
            assert!(portaki_test_utils::conformance::TEMPLATE_MARKERS
                .iter()
                .any(|marker| tagline.starts_with(marker)));

            let fr = json(&dest.path().join("i18n/fr-FR.json"));
            assert_eq!(fr["module.displayName"], "concierge");
            let lib = fs::read_to_string(dest.path().join("src/lib.rs")).expect("lib");
            assert!(lib.contains("author = \"TODO\""), "{template}");
        }
    }

    #[test]
    fn the_answers_fill_both_files_escaped() {
        let mut input =
            std::io::Cursor::new("Le \"Concierge\"\nTout \\ en un.\nAccueil sans clé\n3\nCyril\n");
        let mut output = Vec::new();
        let answers = ask(
            &mut input,
            &mut output,
            "concierge",
            None,
            Answers::default(),
        )
        .expect("answers");
        let dest = scaffold("default-module", &answers);

        let fr = json(&dest.path().join("i18n/fr-FR.json"));
        let en = json(&dest.path().join("i18n/en-US.json"));
        assert_eq!(fr["module.displayName"], "Le \"Concierge\"");
        assert_eq!(en["module.displayName"], "Le \"Concierge\"");
        assert_eq!(fr["module.description"], "Tout \\ en un.");
        // The label of the sheet keeps the id.
        assert_eq!(fr["nav.main"], "concierge");
        let lib = fs::read_to_string(dest.path().join("src/lib.rs")).expect("lib");
        assert!(lib.contains("author = \"Cyril\""));
        let listing = json(&dest.path().join("listing.json"));
        assert_eq!(listing["description"]["fr"], "Tout \\ en un.");
        assert_eq!(listing["tagline"]["fr"], "Accueil sans clé");
        assert_eq!(listing["category"], "around");
    }

    #[test]
    fn enter_everywhere_keeps_the_template_and_the_git_author() {
        let mut input = std::io::Cursor::new("\n\n\n\n\n");
        let answers = ask(
            &mut input,
            &mut Vec::new(),
            "concierge",
            Some("Cyril".into()),
            Answers::default(),
        )
        .expect("answers");

        assert_eq!(
            answers,
            Answers {
                author: Some("Cyril".into()),
                ..Answers::default()
            }
        );
    }

    #[test]
    fn a_tagline_too_long_is_asked_again() {
        let long = "a".repeat(91);
        let mut input = std::io::Cursor::new(format!("\n\n{long}\nCourte\nnope\nformalities\n\n"));
        let mut output = Vec::new();
        let answers = ask(
            &mut input,
            &mut output,
            "concierge",
            None,
            Answers::default(),
        )
        .expect("answers");

        assert_eq!(answers.tagline.as_deref(), Some("Courte"));
        assert_eq!(answers.category, Some(Category::Formalities));
        let shown = String::from_utf8(output).unwrap();
        assert!(shown.contains("91 characters — 90 at most"), "{shown}");
        assert!(shown.contains("no category \"nope\""), "{shown}");
    }

    /// An option answers its question: it is not asked.
    #[test]
    fn options_are_not_asked_again() {
        let preset = Answers {
            display_name: Some("Concierge".into()),
            description: Some("Un module.".into()),
            tagline: Some("Une accroche.".into()),
            category: Some(Category::Arrival),
            author: None,
        };
        let mut input = std::io::Cursor::new("Cyril\n");
        let mut output = Vec::new();
        let answers =
            ask(&mut input, &mut output, "concierge", None, preset.clone()).expect("answers");

        assert_eq!(
            answers,
            Answers {
                author: Some("Cyril".into()),
                ..preset
            }
        );
        let shown = String::from_utf8(output).unwrap();
        assert!(!shown.contains("tagline"), "{shown}");
    }
}
