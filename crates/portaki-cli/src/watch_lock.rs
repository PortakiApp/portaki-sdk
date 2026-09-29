//! One `--watch` session at a time.
//!
//! # Why
//!
//! A `--watch` session compiles, deploys and redeploys endlessly. Two of them running together
//! tread on each other: they push two different modules into the same sandbox in turn, and each
//! undoes what the other has just done. One is rarely started knowingly — it gets forgotten in a
//! tab, and another one gets started elsewhere.
//!
//! # What would make this lock worse than the problem
//!
//! A lock that can no longer be taken over. `--watch` is ended with ctrl-c, and nothing runs
//! then — the file outlives the process. Without a takeover, the first interruption would condemn
//! the command until someone guessed that a file has to be deleted.
//!
//! The lock therefore says **who** holds it, and the takeover is automatic as soon as that
//! process no longer exists.

use std::path::PathBuf;

use anyhow::{Context, Result};

/// The file name, next to the credentials: the lock holds for this person on this machine, not
/// for a repository.
const LOCK_FILE: &str = "watch.lock";

/// What a lock says about its holder.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Holder {
    pid: u32,
    module: String,
}

/// The lock while it is held, handed back when the session ends.
pub struct WatchLock {
    path: PathBuf,
}

impl WatchLock {
    /// Where it lives, for whoever has to hand it back from elsewhere.
    ///
    /// Ctrl-c unwinds nothing: without an explicit hand-back, the file would outlive the session
    /// until the next run found it stale. Harmless, but needlessly obscure.
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }
}

impl Drop for WatchLock {
    fn drop(&mut self) {
        // A lock we fail to delete will be taken over as stale on the next run: nothing to
        // report here, and above all nothing to fail on.
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Takes the lock, or says who holds it.
pub fn acquire(module: &str) -> Result<WatchLock> {
    let directory = crate::auth::config_dir()?;
    std::fs::create_dir_all(&directory)
        .with_context(|| format!("create {}", directory.display()))?;
    let path = directory.join(LOCK_FILE);

    if let Some(holder) = live_holder(&path) {
        anyhow::bail!(
            "a portaki dev session is already running on {} (pid {}) — stop it first",
            holder.module,
            holder.pid
        );
    }

    // The file may be left over from an interrupted session: a `create_new` on its own would
    // then fail forever. We only delete it once we have established that nobody holds it.
    let _ = std::fs::remove_file(&path);
    write(&path, module)?;
    Ok(WatchLock { path })
}

/// `create_new`: two simultaneous runs cannot both succeed.
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

/// The lock's holder, if it still exists.
///
/// An unreadable file reads as no lock at all: better to take over a lock we do not understand
/// than to refuse the command over a damaged file.
fn live_holder(path: &std::path::Path) -> Option<Holder> {
    let raw = std::fs::read_to_string(path).ok()?;
    let holder: Holder = serde_json::from_str(&raw).ok()?;
    alive(holder.pid).then_some(holder)
}

/// Is that process still running, and is it really a `portaki`?
///
/// Both questions in a single call: a PID gets recycled, and a forgotten lock would end up
/// naming one that belongs to another program. Checking existence alone would then have the
/// command refused in the name of a process that never locked anything.
///
/// `ps` rather than a system call: it gives the name at the same time as the existence, and this
/// check only happens when a lock file exists — never on the normal path.
fn alive(pid: u32) -> bool {
    let Ok(output) = std::process::Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "comm="])
        .output()
    else {
        // Without `ps`, nothing can be asserted. Treating the lock as live is the safe choice:
        // refusing a second session costs a message, letting two of them run costs a deploy
        // that overwrites another.
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

    /// The current process is alive, and it really is a `portaki` — the test binary is named
    /// after the crate.
    #[test]
    fn a_running_process_holds_its_lock() {
        assert!(alive(std::process::id()));
    }

    /// A PID that does not exist holds nothing: without that, an interrupted session would
    /// condemn the command until someone guessed that a file has to be deleted.
    #[test]
    fn a_dead_process_holds_nothing() {
        // PID 0 is never an ordinary process; `ps -p 0` fails everywhere.
        assert!(!alive(0));
    }

    /// A PID recycled by another program must not hold our lock.
    #[test]
    fn a_recycled_pid_belonging_to_another_program_holds_nothing() {
        // 1 is `init`/`launchd`: alive, and never `portaki`.
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

    /// The lock names the module and the PID: that is what the second session will display.
    #[test]
    fn a_held_lock_names_who_holds_it() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(LOCK_FILE);
        write(&path, "wifi-guest").unwrap();

        let holder = live_holder(&path).expect("le processus courant tient le verrou");

        assert_eq!(holder.module, "wifi-guest");
        assert_eq!(holder.pid, std::process::id());
    }

    /// `create_new`: the second run cannot overwrite the first.
    #[test]
    fn two_writers_cannot_both_take_it() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(LOCK_FILE);

        write(&path, "first").unwrap();

        assert!(write(&path, "second").is_err());
    }
}
