//! Which module a command acts on, when a repository holds several.
//!
//! The platform's GitHub App does not read the code: it is the CLI, which has the files at hand,
//! that recognises a monorepo. The rule is the one `portaki ci modules` uses — modules under
//! `modules/*/` — and a single-module repository sees nothing change.
//!
//! Every command that acts on a module goes through here, with the same flags
//! ([`ModuleArgs`]): `--module <id>` and `--all`.

use std::io::{BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::ui;

use crate::manifest::source::{is_module, module_id as manifest_id};

/// The directory where a multi-module repository keeps them.
const MODULES_DIR: &str = "modules";

/// `--module <id>` / `--all`, the same ones on every command that acts on a module.
#[derive(Debug, Clone, Default, clap::Args)]
pub struct ModuleArgs {
    /// In a repository holding several modules, the one to act on.
    #[arg(long, value_name = "ID", conflicts_with = "all")]
    pub module: Option<String>,
    /// Every module of the repository.
    #[arg(long)]
    pub all: bool,
}

impl ModuleArgs {
    /// The modules acted on.
    pub fn resolve(&self) -> Result<Vec<Member>> {
        resolve(self.module.as_deref(), Some(self.all))
    }

    /// The one module acted on, for a command that handles only one at a time (`dev`, `logs`).
    pub fn one(&self, command: &str) -> Result<Member> {
        if self.all {
            return Err(crate::exit::usage(crate::tr!(
                "portaki {command} acts on one module at a time — pass --module <id> instead of --all",
                "portaki {command} agit sur un module à la fois — passez --module <id> au lieu de --all"
            )));
        }
        resolve(self.module.as_deref(), None)?
            .into_iter()
            .next()
            .context("no module here")
    }

    /// Each module acted on, from its root, in order; returns to the starting directory.
    pub fn for_each(&self, mut run: impl FnMut(&Member) -> Result<()>) -> Result<()> {
        let start = std::env::current_dir().context("current_dir")?;
        let chosen = self.resolve()?;
        let many = chosen.len() > 1;
        let mut ran = Ok(());
        for member in &chosen {
            if many {
                ui::rule(&member.id);
            }
            enter(member)?;
            ran = run(member).with_context(|| format!("module {}", member.id));
            if ran.is_err() {
                break;
            }
        }
        std::env::set_current_dir(&start).context("return to the starting directory")?;
        ran
    }
}

/// A module of the repository: its id and its root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    pub id: String,
    pub root: PathBuf,
}

/// The modules kept under `modules/*/` of the first ancestor that holds any, sorted by id.
///
/// Walking upwards: run from `modules/nuki`, the repository is still the same one. The root
/// itself does not count — a single-module repository is not a monorepo.
pub fn members(start: &Path) -> Vec<Member> {
    start
        .ancestors()
        .map(nested_members)
        .find(|found| !found.is_empty())
        .unwrap_or_default()
}

fn nested_members(repo: &Path) -> Vec<Member> {
    let Ok(entries) = std::fs::read_dir(repo.join(MODULES_DIR)) else {
        return Vec::new();
    };
    let mut found: Vec<Member> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|root| is_module(root))
        .map(|root| Member {
            id: manifest_id(&root).unwrap_or_else(|| {
                root.file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_default()
            }),
            root,
        })
        .collect();
    found.sort_by(|a, b| a.id.cmp(&b.id));
    found
}

/// What can be decided without asking anyone.
#[derive(Debug, PartialEq, Eq)]
enum Resolved {
    Chosen(Vec<Member>),
    /// Several candidates, no hint: we have to ask.
    Ambiguous,
}

/// The decision on its own, with no terminal, so that it can be checked.
///
/// Outside a monorepo, the current directory is the module, as before. Inside a monorepo:
/// `--all`, then `--module`, then the module one is standing in, then the only one there is.
fn decide(cwd: &Path, members: &[Member], module: Option<&str>, all: bool) -> Result<Resolved> {
    if members.is_empty() {
        let own = manifest_id(cwd);
        if let (Some(wanted), Some(own)) = (module, own.as_deref()) {
            if wanted != own {
                return Err(crate::exit::usage(crate::tr!(
                    "unknown module: {wanted} — this directory is {own}",
                    "module inconnu : {wanted} — ce dossier est {own}"
                )));
            }
        }
        return Ok(Resolved::Chosen(vec![Member {
            id: own.unwrap_or_default(),
            root: cwd.to_path_buf(),
        }]));
    }
    if all {
        return Ok(Resolved::Chosen(members.to_vec()));
    }
    if let Some(wanted) = module {
        return members
            .iter()
            .find(|member| member.id == wanted)
            .map(|member| Resolved::Chosen(vec![member.clone()]))
            .ok_or_else(|| {
                crate::exit::usage(crate::tr!(
                    "unknown module: {wanted} — {}",
                    "module inconnu : {wanted} — {}",
                    ids(members)
                ))
            });
    }
    if let Some(inside) = members.iter().find(|member| cwd.starts_with(&member.root)) {
        return Ok(Resolved::Chosen(vec![inside.clone()]));
    }
    if let [only] = members {
        return Ok(Resolved::Chosen(vec![only.clone()]));
    }
    Ok(Resolved::Ambiguous)
}

/// The modules the command acts on, asking which one when nothing settles it.
///
/// Outside a terminal, no question: a CI that waited for an answer would wait until its own
/// timeout. Not under `--json` either: the question would go out on stdout. The error — a usage
/// one, code 2 — lists the ids, so that they can be copied into `--module`.
///
/// `all` is `None` for a command that acts on a single module (`dev`, `link`).
pub fn resolve(module: Option<&str>, all: Option<bool>) -> Result<Vec<Member>> {
    let cwd = std::env::current_dir().context("current_dir")?;
    let members = members(&cwd);
    match decide(&cwd, &members, module, all == Some(true))? {
        Resolved::Chosen(chosen) => Ok(chosen),
        Resolved::Ambiguous if std::io::stdin().is_terminal() && !ui::json() => {
            ask(&members).map(|m| vec![m])
        }
        Resolved::Ambiguous => Err(crate::exit::usage(crate::tr!(
            "this repository holds several modules — pass --module <id>{}: {}",
            "ce dépôt porte plusieurs modules — passez --module <id>{} : {}",
            if all.is_some() {
                crate::tr!(" or --all", " ou --all")
            } else {
                String::new()
            },
            ids(&members)
        ))),
    }
}

fn ask(members: &[Member]) -> Result<Member> {
    let numbers: Vec<String> = (1..=members.len()).map(|n| n.to_string()).collect();
    let rows: Vec<(&str, &str)> = numbers
        .iter()
        .zip(members)
        .map(|(number, member)| (number.as_str(), member.id.as_str()))
        .collect();
    ui::list("modules", &rows);
    print!("    {} ", crate::tr!("which one?", "lequel ?"));
    std::io::stdout().flush()?;
    let mut answer = String::new();
    std::io::stdin().lock().read_line(&mut answer)?;
    let answer = answer.trim();
    members
        .iter()
        .enumerate()
        .find(|(index, member)| member.id == answer || (index + 1).to_string() == answer)
        .map(|(_, member)| member.clone())
        .with_context(|| format!("no module {answer:?} — {}", ids(members)))
}

fn ids(members: &[Member]) -> String {
    members
        .iter()
        .map(|member| member.id.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Moves to the module's root: the commands read the current directory, and so does `cargo`.
pub fn enter(member: &Member) -> Result<()> {
    std::env::set_current_dir(&member.root)
        .with_context(|| format!("enter {}", member.root.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::source::MODULE_MANIFEST;
    use std::fs;

    fn module(dir: &Path, id: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(
            dir.join(MODULE_MANIFEST),
            format!(r#"{{"id":"{id}","version":"0.1.0"}}"#),
        )
        .unwrap();
    }

    /// The layout of `portaki-modules`: a workspace at the root, the modules underneath.
    fn monorepo() -> tempfile::TempDir {
        let repo = tempfile::tempdir().unwrap();
        module(&repo.path().join("modules/nuki"), "nuki");
        module(&repo.path().join("modules/access-guide"), "access-guide");
        fs::create_dir_all(repo.path().join("modules/not-a-module")).unwrap();
        repo
    }

    fn chosen(resolved: Resolved) -> Vec<String> {
        match resolved {
            Resolved::Chosen(members) => members.into_iter().map(|m| m.id).collect(),
            Resolved::Ambiguous => panic!("attendu un choix"),
        }
    }

    #[test]
    fn a_monorepo_is_found_from_its_root_and_from_inside_a_module() {
        let repo = monorepo();

        let ids: Vec<String> = members(repo.path()).into_iter().map(|m| m.id).collect();
        assert_eq!(ids, vec!["access-guide", "nuki"]);
        assert_eq!(members(&repo.path().join("modules/nuki/src")).len(), 2);
    }

    /// A single-module repository: nothing changes, the current directory is the module.
    #[test]
    fn a_single_module_repository_is_not_a_monorepo() {
        let repo = tempfile::tempdir().unwrap();
        module(repo.path(), "weather");

        assert!(members(repo.path()).is_empty());
        assert_eq!(
            chosen(decide(repo.path(), &[], None, false).unwrap()),
            vec!["weather"]
        );
        assert!(decide(repo.path(), &[], Some("nuki"), false).is_err());
    }

    #[test]
    fn the_root_of_a_monorepo_asks_unless_told() {
        let repo = monorepo();
        let found = members(repo.path());

        assert_eq!(
            decide(repo.path(), &found, None, false).unwrap(),
            Resolved::Ambiguous
        );
        assert_eq!(
            chosen(decide(repo.path(), &found, Some("nuki"), false).unwrap()),
            vec!["nuki"]
        );
        assert_eq!(
            chosen(decide(repo.path(), &found, None, true).unwrap()),
            vec!["access-guide", "nuki"]
        );
        let unknown = decide(repo.path(), &found, Some("wifi"), false).unwrap_err();
        assert!(
            unknown.to_string().contains("access-guide, nuki"),
            "{unknown}"
        );
        assert_eq!(crate::exit::code(&unknown), 2);
    }

    #[test]
    fn inside_a_module_that_module_is_the_answer() {
        let repo = monorepo();
        let found = members(repo.path());
        let inside = repo.path().join("modules/nuki/src");

        assert_eq!(
            chosen(decide(&inside, &found, None, false).unwrap()),
            vec!["nuki"]
        );
    }
}
