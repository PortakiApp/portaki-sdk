//! Une seule session `--watch` à la fois.
//!
//! # Pourquoi
//!
//! Une session `--watch` compile, déploie et redéploie sans fin. Deux qui tournent ensemble se
//! marchent dessus : elles poussent tour à tour deux modules différents dans le même bac à
//! sable, et chacune défait ce que l'autre vient de faire. On la lance rarement en connaissance
//! de cause — on l'oublie dans un onglet, et on en relance une ailleurs.
//!
//! # Ce qui rendrait ce verrou pire que le problème
//!
//! Un verrou qu'on ne peut plus reprendre. `--watch` se termine à coups de ctrl-c, et rien ne
//! s'exécute alors — le fichier survit au processus. Sans reprise, la première interruption
//! condamnerait la commande jusqu'à ce que quelqu'un devine qu'il faut effacer un fichier.
//!
//! Le verrou dit donc **qui** le tient, et la reprise est automatique dès que ce processus
//! n'existe plus.

use std::path::PathBuf;

use anyhow::{Context, Result};

/// Le nom du fichier, à côté des identifiants : le verrou vaut pour cette personne sur cette
/// machine, pas pour un dépôt.
const LOCK_FILE: &str = "watch.lock";

/// Ce qu'un verrou dit de son détenteur.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Holder {
    pid: u32,
    module: String,
}

/// Le verrou tenu, rendu à la fin de la session.
pub struct WatchLock {
    path: PathBuf,
}

impl WatchLock {
    /// Où il vit, pour qui doit le rendre depuis ailleurs.
    ///
    /// Ctrl-c ne déroule rien : sans un rendu explicite, le fichier survivrait jusqu'à ce que
    /// le lancement suivant le constate périmé. Sans gravité, mais inutilement obscur.
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }
}

impl Drop for WatchLock {
    fn drop(&mut self) {
        // Un verrou qu'on n'arrive pas à effacer sera repris comme périmé au prochain
        // lancement : rien à signaler ici, et surtout rien à faire échouer.
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Prend le verrou, ou dit qui le tient.
pub fn acquire(module: &str) -> Result<WatchLock> {
    let directory = crate::auth::config_dir()?;
    std::fs::create_dir_all(&directory)
        .with_context(|| format!("create {}", directory.display()))?;
    let path = directory.join(LOCK_FILE);

    if let Some(holder) = live_holder(&path) {
        anyhow::bail!(
            "portaki dev --watch is already running on {} (pid {}) — stop it first, or run \
             this one without --watch",
            holder.module,
            holder.pid
        );
    }

    // Le fichier peut rester d'une session interrompue : un `create_new` seul échouerait alors
    // pour toujours. On ne l'efface qu'après avoir établi que personne ne le tient.
    let _ = std::fs::remove_file(&path);
    write(&path, module)?;
    Ok(WatchLock { path })
}

/// `create_new` : deux lancements simultanés ne peuvent pas réussir tous les deux.
fn write(path: &std::path::Path, module: &str) -> Result<()> {
    use std::io::Write as _;
    let holder = Holder {
        pid: std::process::id(),
        module: module.to_string(),
    };
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| {
            format!(
                "another portaki took the watch lock at {} just now",
                path.display()
            )
        })?;
    file.write_all(serde_json::to_string(&holder)?.as_bytes())
        .with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

/// Le détenteur du verrou, s'il existe encore.
///
/// Un fichier illisible se lit comme une absence : mieux vaut reprendre un verrou qu'on ne
/// comprend pas que refuser la commande pour un fichier abîmé.
fn live_holder(path: &std::path::Path) -> Option<Holder> {
    let raw = std::fs::read_to_string(path).ok()?;
    let holder: Holder = serde_json::from_str(&raw).ok()?;
    alive(holder.pid).then_some(holder)
}

/// Ce processus tourne-t-il encore, et est-ce bien un `portaki` ?
///
/// Les deux questions en un seul appel : un PID est réattribué, et un verrou oublié finirait
/// par en désigner un qui appartient à un autre programme. Vérifier la seule existence ferait
/// alors refuser la commande au nom d'un processus qui n'a jamais rien verrouillé.
///
/// `ps` plutôt qu'un appel système : il rend le nom en même temps que l'existence, et cette
/// vérification n'a lieu que lorsqu'un fichier de verrou existe — jamais sur le chemin normal.
fn alive(pid: u32) -> bool {
    let Ok(output) = std::process::Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "comm="])
        .output()
    else {
        // Sans `ps`, on ne peut rien affirmer. Tenir le verrou pour vivant est le choix sûr :
        // refuser une seconde session coûte un message, en laisser tourner deux coûte un
        // déploiement qui en écrase un autre.
        return true;
    };
    if !output.status.success() {
        return false;
    }
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .rsplit('/')
        .next()
        .is_some_and(|name| name.starts_with("portaki"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Le processus courant est vivant, et c'est bien un `portaki` — le binaire de test porte
    /// le nom de la crate.
    #[test]
    fn a_running_process_holds_its_lock() {
        assert!(alive(std::process::id()));
    }

    /// Un PID qui n'existe pas ne tient rien : sans quoi une session interrompue condamnerait
    /// la commande jusqu'à ce que quelqu'un devine qu'il faut effacer un fichier.
    #[test]
    fn a_dead_process_holds_nothing() {
        // PID 0 n'est jamais un processus ordinaire ; `ps -p 0` échoue partout.
        assert!(!alive(0));
    }

    /// Un PID réattribué à un autre programme ne doit pas tenir notre verrou.
    #[test]
    fn a_recycled_pid_belonging_to_another_program_holds_nothing() {
        // 1 est `init`/`launchd` : vivant, et jamais `portaki`.
        assert!(!alive(1));
    }

    #[test]
    fn a_damaged_lock_reads_as_free() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(LOCK_FILE);
        std::fs::write(&path, "pas du json").unwrap();

        assert!(live_holder(&path).is_none());
    }

    #[test]
    fn a_lock_naming_a_dead_process_reads_as_free() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(LOCK_FILE);
        std::fs::write(&path, r#"{"pid":0,"module":"weather"}"#).unwrap();

        assert!(live_holder(&path).is_none());
    }

    /// Le verrou nomme le module et le PID : c'est ce que la seconde session affichera.
    #[test]
    fn a_held_lock_names_who_holds_it() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(LOCK_FILE);
        write(&path, "wifi-guest").unwrap();

        let holder = live_holder(&path).expect("le processus courant tient le verrou");

        assert_eq!(holder.module, "wifi-guest");
        assert_eq!(holder.pid, std::process::id());
    }

    /// `create_new` : le second lancement ne peut pas écraser le premier.
    #[test]
    fn two_writers_cannot_both_take_it() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(LOCK_FILE);

        write(&path, "first").unwrap();

        assert!(write(&path, "second").is_err());
    }
}
