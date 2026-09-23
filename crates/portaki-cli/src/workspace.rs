//! Quel module une commande vise, quand un dépôt en porte plusieurs.
//!
//! La GitHub App de la plateforme ne lit pas le code : c'est le CLI, qui a les fichiers sous la
//! main, qui reconnaît un monorepo. La règle est celle de `portaki ci modules` — des
//! `portaki.module.json` sous `modules/*/` — et un dépôt à un seul module ne voit rien changer.

use std::io::{BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::ui;

/// Le manifeste qui fait d'un dossier un module.
const MODULE_MANIFEST: &str = "portaki.module.json";

/// Le dossier où un dépôt multi-modules les range.
const MODULES_DIR: &str = "modules";

/// Un module du dépôt : son identifiant et sa racine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    pub id: String,
    pub root: PathBuf,
}

/// Les modules rangés sous `modules/*/` du premier ancêtre qui en porte, triés par id.
///
/// En remontant : lancé depuis `modules/nuki`, le dépôt reste le même. La racine elle-même ne
/// compte pas — un dépôt à un seul module n'est pas un monorepo.
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
        .filter(|root| root.join(MODULE_MANIFEST).is_file())
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

fn manifest_id(root: &Path) -> Option<String> {
    let raw = std::fs::read_to_string(root.join(MODULE_MANIFEST)).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&raw).ok()?;
    parsed.get("id")?.as_str().map(str::to_string)
}

/// Ce qu'on peut décider sans demander à personne.
#[derive(Debug, PartialEq, Eq)]
enum Resolved {
    Chosen(Vec<Member>),
    /// Plusieurs candidats, aucun indice : il faut demander.
    Ambiguous,
}

/// La décision seule, sans terminal, pour être vérifiable.
///
/// Hors monorepo, le dossier courant est le module, comme avant. Dans un monorepo : `--all`,
/// puis `--module`, puis le module dans lequel on se trouve, puis le seul qui existe.
fn decide(cwd: &Path, members: &[Member], module: Option<&str>, all: bool) -> Result<Resolved> {
    if members.is_empty() {
        let own = manifest_id(cwd);
        if let (Some(wanted), Some(own)) = (module, own.as_deref()) {
            if wanted != own {
                anyhow::bail!("unknown module: {wanted} — this directory is {own}");
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
            .with_context(|| format!("unknown module: {wanted} — {}", ids(members)));
    }
    if let Some(inside) = members.iter().find(|member| cwd.starts_with(&member.root)) {
        return Ok(Resolved::Chosen(vec![inside.clone()]));
    }
    if let [only] = members {
        return Ok(Resolved::Chosen(vec![only.clone()]));
    }
    Ok(Resolved::Ambiguous)
}

/// Les modules visés par la commande, en demandant lequel quand rien ne permet de trancher.
///
/// Hors terminal, pas de question : une CI qui attendrait une réponse attendrait jusqu'à son
/// délai. L'erreur liste les ids, pour qu'on puisse les recopier dans `--module`.
///
/// `all` vaut `None` pour une commande qui ne vise qu'un module (`dev`, `link`).
pub fn resolve(module: Option<&str>, all: Option<bool>) -> Result<Vec<Member>> {
    let cwd = std::env::current_dir().context("current_dir")?;
    let members = members(&cwd);
    match decide(&cwd, &members, module, all == Some(true))? {
        Resolved::Chosen(chosen) => Ok(chosen),
        Resolved::Ambiguous if std::io::stdin().is_terminal() => ask(&members).map(|m| vec![m]),
        Resolved::Ambiguous => anyhow::bail!(
            "this repository holds several modules — pass --module <id>{}: {}",
            if all.is_some() { " or --all" } else { "" },
            ids(&members)
        ),
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
    print!("    which one? ");
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

/// Se place à la racine du module : les commandes lisent le dossier courant, `cargo` aussi.
pub fn enter(member: &Member) -> Result<()> {
    std::env::set_current_dir(&member.root)
        .with_context(|| format!("enter {}", member.root.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn module(dir: &Path, id: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(
            dir.join(MODULE_MANIFEST),
            format!(r#"{{"id":"{id}","version":"0.1.0"}}"#),
        )
        .unwrap();
    }

    /// La disposition de `portaki-modules` : un workspace à la racine, les modules dessous.
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

    /// Un dépôt à un module : rien ne change, le dossier courant est le module.
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
        let unknown = decide(repo.path(), &found, Some("wifi"), false)
            .unwrap_err()
            .to_string();
        assert!(unknown.contains("access-guide, nuki"), "{unknown}");
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
