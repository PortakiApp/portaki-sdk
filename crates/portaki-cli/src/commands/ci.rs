//! `portaki ci` — ce qu'un workflow avait jusqu'ici à faire en bash.
//!
//! Un dépôt de modules pilotait sa CI avec deux cents lignes de `bash`, `jq` et `curl` :
//! découvrir les modules changés, résoudre le CLI à installer, annoncer un run. Ces trois
//! choses sont des questions sur un module — le CLI en sait plus qu'un script, et il est déjà
//! installé sur le runner.
//!
//! Chaque sous-commande écrit sur la sortie standard **et** dans `GITHUB_OUTPUT` quand il
//! existe : lisible à la main, consommable par une étape suivante, sans deuxième forme à tenir.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use crate::manifest::generator::{MANIFEST_VERSION, SDUI_SCHEMA_VERSION};
use crate::ui;

/// Le manifeste qui fait d'un dossier un module.
const MODULE_MANIFEST: &str = "portaki.module.json";

/// Le dossier où un dépôt multi-modules les range.
const MODULES_DIR: &str = "modules";

/// Ce qui, changé, oblige à tout reconstruire : le socle commun à tous les modules.
const SHARED_PATHS: [&str; 5] = [
    "Cargo.toml",
    "Cargo.lock",
    ".cargo/",
    "rust-toolchain",
    "rust-toolchain.toml",
];

#[derive(Debug, Parser)]
/// Arguments for `portaki ci`.
pub struct CiArgs {
    #[command(subcommand)]
    pub command: CiCommand,
}

#[derive(Debug, Subcommand)]
/// Les questions qu'un workflow pose sur un dépôt de modules.
pub enum CiCommand {
    /// List the modules to build — all of them, or only those a change touched.
    Modules(ModulesArgs),
    /// Print the Portaki SDK this checkout actually resolves to, as a cache key.
    SdkVersion(SdkVersionArgs),
    /// Warn about what will age badly: an old SDK, a manifest behind the host.
    Check(CheckArgs),
    /// Print this module's id and version — what a workflow needs to name a release.
    Info(InfoArgs),
}

/// Runs `portaki ci`.
pub async fn run(args: CiArgs) -> Result<()> {
    match args.command {
        CiCommand::Modules(args) => modules(args),
        CiCommand::SdkVersion(args) => sdk_version(args),
        CiCommand::Check(args) => check(args).await,
        CiCommand::Info(args) => info(args),
    }
}

#[derive(Debug, Parser)]
/// Arguments for `portaki ci info`.
pub struct InfoArgs {
    /// Module root (defaults to the current directory).
    #[arg(long)]
    pub root: Option<PathBuf>,
}

/// L'identité du module, pour un workflow qui doit la nommer.
///
/// Sans elle, une action composite en était réduite à extraire la version du manifeste avec
/// `python3` ou `jq` — une dépendance de plus sur le runner, pour un champ que le CLI lit déjà.
fn info(args: InfoArgs) -> Result<()> {
    let root = args
        .root
        .clone()
        .map(Ok)
        .unwrap_or_else(std::env::current_dir)
        .context("resolve the module root")?;
    let manifest = root.join(MODULE_MANIFEST);
    let raw = std::fs::read_to_string(&manifest)
        .with_context(|| format!("read {} — run from the module root", manifest.display()))?;
    let parsed: serde_json::Value =
        serde_json::from_str(&raw).with_context(|| format!("parse {}", manifest.display()))?;

    let field = |key: &str| {
        parsed
            .get(key)
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_string()
    };
    let (id, version) = (field("id"), field("version"));
    if id.is_empty() || version.is_empty() {
        anyhow::bail!("{MODULE_MANIFEST} carries no id or no version");
    }

    emit_outputs(&[("id", &id), ("version", &version)])?;

    if ui::plain() {
        // Deux lignes, dans un ordre fixe : `read id version < <(portaki --plain ci info)`.
        println!("{id}");
        println!("{version}");
        return Ok(());
    }
    ui::header("portaki ci info", "What this module calls itself.");
    ui::field("id", &id);
    ui::field("version", &version);
    ui::blank();
    Ok(())
}

#[derive(Debug, Parser)]
/// Arguments for `portaki ci modules`.
pub struct ModulesArgs {
    /// Repository root (defaults to the current directory).
    #[arg(long)]
    pub root: Option<PathBuf>,
    /// Keep only the modules changed since this git ref.
    #[arg(long)]
    pub changed_since: Option<String>,
    /// Restrict to these modules, whatever changed.
    #[arg(long, value_delimiter = ',')]
    pub only: Vec<String>,
}

/// Ce qu'on a trouvé, et pourquoi.
///
/// La raison n'est pas de la décoration : « rien n'a changé » et « je n'ai pas su comparer »
/// produisent la même liste vide et n'appellent pas la même réaction.
struct Selection {
    modules: Vec<String>,
    reason: &'static str,
}

fn modules(args: ModulesArgs) -> Result<()> {
    let root = args
        .root
        .clone()
        .map(Ok)
        .unwrap_or_else(std::env::current_dir)
        .context("resolve the repository root")?;

    let known = discover(&root)?;
    if known.is_empty() {
        anyhow::bail!(
            "no module found — expected {MODULE_MANIFEST} here, or one under {MODULES_DIR}/*/"
        );
    }

    let selection = select(&root, &known, &args)?;
    let json = serde_json::to_string(&selection.modules)?;

    // Le workflow lit `modules` pour sa matrice et `any` pour sauter les jobs : sans `any`, une
    // matrice vide fait échouer le job au lieu de le passer.
    emit_outputs(&[
        ("modules", &json),
        ("any", &(!selection.modules.is_empty()).to_string()),
        ("reason", selection.reason),
    ])?;

    if ui::plain() {
        println!("{json}");
        return Ok(());
    }

    ui::header(
        "portaki ci modules",
        "The modules this run should build, and why that is the list.",
    );
    if selection.modules.is_empty() {
        ui::skipped(format!("nothing to build ({})", selection.reason));
    } else {
        ui::detail(selection.reason);
        // La colonne dit la version déclarée plutôt que la raison : celle-ci vaut pour toute la
        // liste, la répéter vingt fois n'apprend rien, et c'est la version qu'on cherche des
        // yeux quand on relit une release.
        let versions: Vec<String> = selection
            .modules
            .iter()
            .map(|name| declared_version(&root, name).unwrap_or_else(|| "?".to_string()))
            .collect();
        let rows: Vec<(&str, &str)> = selection
            .modules
            .iter()
            .zip(&versions)
            .map(|(name, version)| (name.as_str(), version.as_str()))
            .collect();
        ui::list("modules", &rows);
    }
    ui::blank();
    Ok(())
}

/// La version qu'un module déclare, pour la montrer en regard de son nom.
fn declared_version(root: &Path, name: &str) -> Option<String> {
    for candidate in [root.join(MODULES_DIR).join(name), root.to_path_buf()] {
        let manifest = candidate.join(MODULE_MANIFEST);
        if let Ok(raw) = std::fs::read_to_string(&manifest) {
            let parsed: serde_json::Value = serde_json::from_str(&raw).ok()?;
            return parsed
                .get("version")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string);
        }
    }
    None
}

/// Les modules du dépôt : `modules/*` s'il y en a, sinon le dossier courant lui-même.
///
/// Les deux dispositions coexistent — un dépôt par module, ou un dépôt qui les rassemble — et
/// aucune n'est déclarée nulle part. C'est la présence du manifeste qui tranche.
fn discover(root: &Path) -> Result<Vec<String>> {
    let nested = root.join(MODULES_DIR);
    if nested.is_dir() {
        let mut found = Vec::new();
        for entry in std::fs::read_dir(&nested)
            .with_context(|| format!("read {}", nested.display()))?
            .flatten()
        {
            if entry.path().join(MODULE_MANIFEST).is_file() {
                found.push(entry.file_name().to_string_lossy().to_string());
            }
        }
        found.sort();
        if !found.is_empty() {
            return Ok(found);
        }
    }
    if root.join(MODULE_MANIFEST).is_file() {
        let name = root
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| ".".to_string());
        return Ok(vec![name]);
    }
    Ok(Vec::new())
}

fn select(root: &Path, known: &[String], args: &ModulesArgs) -> Result<Selection> {
    if !args.only.is_empty() {
        let mut chosen = Vec::new();
        for name in &args.only {
            let name = name.trim();
            if name.is_empty() {
                continue;
            }
            if !known.iter().any(|candidate| candidate == name) {
                anyhow::bail!("unknown module: {name}");
            }
            chosen.push(name.to_string());
        }
        return Ok(Selection {
            modules: chosen,
            reason: "asked for",
        });
    }

    let Some(base) = args.changed_since.as_deref() else {
        return Ok(Selection {
            modules: known.to_vec(),
            reason: "no base to compare against",
        });
    };

    // Une base absente ou nulle veut dire « première poussée », « force-push » ou « clone
    // superficiel ». Tout reconstruire y coûte des minutes ; ne rien reconstruire y perdrait
    // une publication, ce qui coûte davantage.
    if base.trim().is_empty() || base.chars().all(|character| character == '0') {
        return Ok(Selection {
            modules: known.to_vec(),
            reason: "no usable base",
        });
    }

    let Some(changed) = changed_paths(root, base) else {
        return Ok(Selection {
            modules: known.to_vec(),
            reason: "base not found",
        });
    };

    if changed.is_empty() {
        return Ok(Selection {
            modules: Vec::new(),
            reason: "nothing changed",
        });
    }

    if changed.iter().any(|path| touches_shared(path)) {
        return Ok(Selection {
            modules: known.to_vec(),
            reason: "the shared workspace changed",
        });
    }

    let mut touched: Vec<String> = known
        .iter()
        .filter(|name| {
            let prefix = format!("{MODULES_DIR}/{name}/");
            changed.iter().any(|path| path.starts_with(&prefix))
        })
        .cloned()
        .collect();
    touched.sort();
    touched.dedup();

    if touched.is_empty() {
        return Ok(Selection {
            modules: Vec::new(),
            reason: "no module was touched",
        });
    }
    Ok(Selection {
        modules: touched,
        reason: "changed",
    })
}

/// Les chemins modifiés depuis `base`, ou `None` si git ne sait pas comparer.
fn changed_paths(root: &Path, base: &str) -> Option<Vec<String>> {
    let output = std::process::Command::new("git")
        .current_dir(root)
        .args(["diff", "--name-only", &format!("{base}...HEAD")])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::to_string)
            .filter(|path| !path.is_empty())
            .collect(),
    )
}

/// Ce chemin appartient-il au socle commun ?
///
/// Comparé sur des préfixes, pas sur une expression : `.github/workflows/ci.yml` ne doit pas
/// déclencher la matrice entière, et une expression un peu large le ferait sans qu'on le voie.
fn touches_shared(path: &str) -> bool {
    SHARED_PATHS.iter().any(|shared| {
        if let Some(directory) = shared.strip_suffix('/') {
            path.starts_with(&format!("{directory}/"))
        } else {
            path == *shared
        }
    })
}

#[derive(Debug, Parser)]
/// Arguments for `portaki ci sdk-version`.
pub struct SdkVersionArgs {
    /// Directory holding the `Cargo.lock` to read (defaults to the current directory).
    #[arg(long)]
    pub root: Option<PathBuf>,
}

/// Le SDK auquel ce checkout se résout, et la version de CLI à installer avec.
///
/// Lu de `Cargo.lock`, pas de `Cargo.toml` : un module peut déclarer le SDK par semver, par
/// branche git ou par chemin, et seul le lock dit ce qui sera réellement compilé.
///
/// La clé rendue est la version seule, parce que le CLI s'installe depuis crates.io —
/// `cargo install portaki-cli@<version>`. Cloner le dépôt du SDK pour l'y compiler coûtait une
/// résolution de branche à chaque run, un cache invalidé à chaque commit du SDK, et un binaire
/// qui n'était celui d'aucune version publiée. La révision reste rendue à titre indicatif :
/// elle dit que le checkout suit une branche, pas une release.
fn sdk_version(args: SdkVersionArgs) -> Result<()> {
    let root = args
        .root
        .clone()
        .map(Ok)
        .unwrap_or_else(std::env::current_dir)
        .context("resolve the checkout root")?;
    let lock = find_lockfile(&root).context(
        "no Cargo.lock found here or above — the SDK a build resolves to is written there",
    )?;
    let text =
        std::fs::read_to_string(&lock).with_context(|| format!("read {}", lock.display()))?;
    let resolved = read_locked_sdk(&text).context("Cargo.lock carries no portaki-sdk")?;

    emit_outputs(&[
        ("version", &resolved.version),
        ("rev", resolved.rev.as_deref().unwrap_or("")),
        ("key", resolved.cache_key()),
    ])?;

    if ui::plain() {
        println!("{}", resolved.cache_key());
        return Ok(());
    }

    ui::header(
        "portaki ci sdk-version",
        "The Portaki SDK this checkout resolves to — the key a CLI cache should use.",
    );
    ui::field("version", &resolved.version);
    if let Some(rev) = &resolved.rev {
        ui::field("rev", rev);
    }
    ui::field("install", format!("portaki-cli@{}", resolved.cache_key()));
    if resolved.rev.is_some() {
        ui::advice("this checkout follows a git branch — the published CLI may lag behind it");
    }
    ui::blank();
    Ok(())
}

/// Le SDK résolu par le lock.
#[derive(Debug, PartialEq, Eq)]
struct LockedSdk {
    version: String,
    /// La révision exacte, quand le SDK vient d'un dépôt git plutôt que de crates.io.
    rev: Option<String>,
}

impl LockedSdk {
    /// La version à installer, et la clé de cache qui va avec — les deux sont la même chose.
    fn cache_key(&self) -> &str {
        &self.version
    }
}

/// Le `Cargo.lock` le plus proche, en remontant : un module d'un dépôt multi-modules partage
/// celui de la racine.
fn find_lockfile(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .map(|directory| directory.join("Cargo.lock"))
        .find(|candidate| candidate.is_file())
}

/// Extrait `portaki-sdk` du lock, sans dépendance de plus.
///
/// Le format est stable et trivial — des blocs `[[package]]` de lignes `clé = "valeur"`. Ajouter
/// un analyseur TOML complet au CLI pour deux champs coûterait plus qu'il ne protège.
fn read_locked_sdk(lock: &str) -> Option<LockedSdk> {
    let mut in_sdk = false;
    let mut version = None;
    let mut source = None;

    for line in lock.lines() {
        let line = line.trim();
        if line == "[[package]]" {
            if in_sdk {
                break;
            }
            version = None;
            source = None;
            continue;
        }
        if let Some(value) = quoted(line, "name") {
            in_sdk = value == "portaki-sdk";
            continue;
        }
        if let Some(value) = quoted(line, "version") {
            version = Some(value.to_string());
        }
        if let Some(value) = quoted(line, "source") {
            source = Some(value.to_string());
        }
        if in_sdk && version.is_some() && line.starts_with("dependencies") {
            break;
        }
    }

    let version = version?;
    if !in_sdk {
        return None;
    }
    let rev = source
        .and_then(|source| source.split_once('#').map(|(_, rev)| rev.to_string()))
        .filter(|rev| !rev.is_empty());
    Some(LockedSdk { version, rev })
}

/// `clé = "valeur"` → `valeur`.
fn quoted<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let rest = line.strip_prefix(key)?.trim_start();
    let rest = rest.strip_prefix('=')?.trim();
    rest.strip_prefix('"')?.strip_suffix('"')
}

#[derive(Debug, Parser)]
/// Arguments for `portaki ci check`.
pub struct CheckArgs {
    /// Module root (defaults to the current directory).
    #[arg(long)]
    pub root: Option<PathBuf>,
    /// Skip the crates.io lookup — for an offline runner.
    #[arg(long)]
    pub offline: bool,
}

/// Ce qui n'empêche rien aujourd'hui et coûtera cher plus tard.
async fn check(args: CheckArgs) -> Result<()> {
    let root = args
        .root
        .clone()
        .map(Ok)
        .unwrap_or_else(std::env::current_dir)
        .context("resolve the module root")?;

    if !ui::plain() {
        ui::header(
            "portaki ci check",
            "What still builds today and will not tomorrow.",
        );
    }

    let mut found = 0;
    let (manifest, _) = crate::manifest::load_manifest(&root, None)?;

    if manifest.manifest_version != MANIFEST_VERSION {
        found += 1;
        annotate(
            Some(MODULE_MANIFEST),
            format!(
                "manifest_version {} — this SDK writes {MANIFEST_VERSION}; rebuild with a current \
                 portaki build",
                manifest.manifest_version
            ),
        );
    }
    for (shell, declared) in [
        ("host", &manifest.ui_schema.host),
        ("guest", &manifest.ui_schema.guest),
    ] {
        if declared != SDUI_SCHEMA_VERSION {
            found += 1;
            annotate(
                Some(MODULE_MANIFEST),
                format!(
                    "ui_schema {shell}={declared} — the shell now renders {SDUI_SCHEMA_VERSION}; \
                     newer primitives will not be available"
                ),
            );
        }
    }

    if args.offline {
        ui::skipped("skipped the crates.io lookup (--offline)");
    } else if let Some(lock) = find_lockfile(&root) {
        let text = std::fs::read_to_string(&lock)?;
        if let Some(resolved) = read_locked_sdk(&text) {
            match latest_sdk().await {
                Ok(latest) if outdated(&resolved.version, &latest) => {
                    found += 1;
                    annotate(
                        Some("Cargo.toml"),
                        format!(
                            "portaki-sdk {} — {latest} is published; \
                             https://github.com/PortakiApp/portaki-sdk/releases",
                            resolved.version
                        ),
                    );
                }
                Ok(_) => {}
                // Un registre injoignable n'est pas un défaut du module : le dire, et continuer.
                Err(failure) => ui::skipped(format!("could not reach crates.io: {failure}")),
            }
        }
    }

    if found == 0 {
        ui::success(format!("{} has nothing ageing", manifest.id));
    }
    if !ui::plain() {
        ui::blank();
    }
    Ok(())
}

/// Écrit l'avertissement là où il sera vu.
///
/// Sous GitHub Actions, la syntaxe `::warning::` l'épingle sur le fichier concerné, dans la vue
/// des changements. Ailleurs, c'est une ligne comme une autre — la même information, sans
/// l'encodage qui ne servirait à personne.
fn annotate(file: Option<&str>, message: impl std::fmt::Display) {
    if std::env::var("GITHUB_ACTIONS").is_ok_and(|value| value == "true") {
        match file {
            Some(file) => println!("::warning file={file}::{message}"),
            None => println!("::warning::{message}"),
        }
        return;
    }
    ui::warn(message);
}

/// La dernière version publiée du SDK.
async fn latest_sdk() -> Result<String> {
    let response = reqwest::Client::new()
        .get("https://crates.io/api/v1/crates/portaki-sdk")
        // crates.io refuse une requête sans agent identifiable, et le dit en 403.
        .header(
            "User-Agent",
            concat!("portaki-cli/", env!("CARGO_PKG_VERSION")),
        )
        .send()
        .await?;
    let body: serde_json::Value = response.json().await?;
    body.pointer("/crate/max_stable_version")
        .or_else(|| body.pointer("/crate/newest_version"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .context("crates.io did not say which version is newest")
}

/// `declared` est-il en retard sur `latest` ?
///
/// Comparé composant par composant, en nombres : `2.10.0` est postérieur à `2.9.0`, ce qu'un
/// ordre lexicographique inverserait.
fn outdated(declared: &str, latest: &str) -> bool {
    parts(latest) > parts(declared)
}

fn parts(version: &str) -> Vec<u64> {
    version
        .split('-')
        .next()
        .unwrap_or(version)
        .split('.')
        .map(|part| part.parse().unwrap_or(0))
        .collect()
}

/// Rend les valeurs disponibles à l'étape suivante du workflow.
fn emit_outputs(pairs: &[(&str, &str)]) -> Result<()> {
    let Ok(path) = std::env::var("GITHUB_OUTPUT") else {
        return Ok(());
    };
    use std::io::Write as _;
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(&path)
        .with_context(|| format!("open {path}"))?;
    for (key, value) in pairs {
        writeln!(file, "{key}={value}").with_context(|| format!("write {key} to {path}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Le lock d'un dépôt qui suit une branche : la version est celle du crate, la révision
    /// celle du commit compilé.
    const GIT_LOCK: &str = r#"
[[package]]
name = "serde"
version = "1.0.0"

[[package]]
name = "portaki-sdk"
version = "2.2.0"
source = "git+https://github.com/PortakiApp/portaki-sdk.git?branch=main#28d522da69b70627f78123d9c42475cb7c595c46"
dependencies = [
 "base64",
]
"#;

    const REGISTRY_LOCK: &str = r#"
[[package]]
name = "portaki-sdk"
version = "2.3.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
"#;

    #[test]
    fn a_git_dependency_still_yields_a_published_version() {
        let resolved = read_locked_sdk(GIT_LOCK).expect("portaki-sdk dans le lock");

        assert_eq!(resolved.version, "2.2.0");
        assert_eq!(
            resolved.rev.as_deref(),
            Some("28d522da69b70627f78123d9c42475cb7c595c46")
        );
        // La clé sert à `cargo install portaki-cli@<clé>` : elle ne porte que la version.
        assert_eq!(resolved.cache_key(), "2.2.0");
    }

    #[test]
    fn a_registry_dependency_has_no_revision() {
        let resolved = read_locked_sdk(REGISTRY_LOCK).expect("portaki-sdk dans le lock");

        assert_eq!(resolved.cache_key(), "2.3.0");
        assert!(resolved.rev.is_none());
    }

    #[test]
    fn a_lock_without_the_sdk_says_so() {
        assert!(read_locked_sdk("[[package]]\nname = \"serde\"\nversion = \"1.0.0\"\n").is_none());
    }

    /// Comparées en nombres : lexicographiquement, `2.9.0` passerait pour postérieur à `2.10.0`.
    #[test]
    fn versions_compare_as_numbers_not_as_text() {
        assert!(outdated("2.9.0", "2.10.0"));
        assert!(!outdated("2.10.0", "2.9.0"));
        assert!(!outdated("2.3.0", "2.3.0"));
        assert!(outdated("2.2.0", "2.3.0"));
    }

    /// Le socle commun fait tout reconstruire — mais un fichier de CI n'en fait pas partie,
    /// sinon la moindre retouche de workflow déclencherait vingt et une publications.
    #[test]
    fn only_the_shared_workspace_fans_out() {
        assert!(touches_shared("Cargo.lock"));
        assert!(touches_shared("Cargo.toml"));
        assert!(touches_shared(".cargo/config.toml"));
        assert!(touches_shared("rust-toolchain.toml"));

        assert!(!touches_shared(".github/workflows/ci.yml"));
        assert!(!touches_shared("modules/weather/Cargo.toml"));
        assert!(!touches_shared("README.md"));
    }

    /// Une disposition n'est déclarée nulle part : c'est le manifeste qui la révèle.
    #[test]
    fn both_repository_layouts_are_recognised() {
        let single = tempfile::tempdir().unwrap();
        std::fs::write(single.path().join(MODULE_MANIFEST), r#"{"id":"weather"}"#).unwrap();
        assert_eq!(discover(single.path()).unwrap().len(), 1);

        let many = tempfile::tempdir().unwrap();
        for name in ["weather", "nuki"] {
            let directory = many.path().join(MODULES_DIR).join(name);
            std::fs::create_dir_all(&directory).unwrap();
            std::fs::write(directory.join(MODULE_MANIFEST), r#"{"id":"x"}"#).unwrap();
        }
        assert_eq!(discover(many.path()).unwrap(), vec!["nuki", "weather"]);
    }

    /// Un dossier sous `modules/` sans manifeste n'est pas un module — `target/`, par exemple.
    #[test]
    fn a_directory_without_a_manifest_is_not_a_module() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join(MODULES_DIR).join("target")).unwrap();

        assert!(discover(root.path()).unwrap().is_empty());
    }
}
