//! Where the CLI keeps its credentials.
//!
//! In a file, `~/.config/portaki/credentials.json`, mode `0600` — and no longer in the system
//! keychain.
//!
//! The keychain was the right choice on paper: encrypted at rest, locked along with the session.
//! It stayed the right choice until we measured what it really costs on macOS — it ties its
//! authorisation to the binary's code identity, and a recompiled binary is a stranger. So a
//! development loop that recompiles asks for the session password again on every pass. A
//! safeguard you run into a hundred times a day ends up being worked around; this one already
//! was, through the environment variable.
//!
//! What the file keeps:
//!
//! - `0600` on the file, `0700` on its directory — on a single-user machine, that is the
//!   protection that actually counts;
//! - outside the repository, under `$XDG_CONFIG_HOME`, so never committed nor caught by a
//!   `git add -A`;
//! - written by atomic rename: an interruption never leaves a truncated file;
//! - never displayed, and `portaki logout` erases it.
//!
//! What it does not keep: encryption at rest. **Hashing is impossible** — a token has to be
//! replayed as it is, and a digest cannot be replayed. Encrypting would need a key, which would
//! have to be kept… in the keychain we have just left. Saying so is better than scrambling the
//! contents to look the part.
//!
//! `PORTAKI_CREDENTIALS=keychain` restores the old behaviour, for whoever prefers it.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};

const SERVICE: &str = "app.portaki.cli";
const ACCESS_ENTRY: &str = "access-token";
const REFRESH_ENTRY: &str = "refresh-token";
/// The origin of a session stored back when sessions were not stored per origin.
const ORIGIN_ENTRY: &str = "origin";

/// A session stored before we recorded its origin: production, the only one there was by default.
const LEGACY_ORIGIN: &str = "https://api.portaki.app";

/// The token set explicitly in the environment, if there is one.
///
/// It wins over everything else, including a CI's OIDC: an explicit choice must outrank a
/// mechanism that switches itself on, otherwise setting this variable would no longer have any
/// visible effect and debugging would turn into a guessing game.
pub fn explicit_token() -> Option<String> {
    std::env::var("PORTAKI_DEV_TOKEN")
        .ok()
        .map(|token| token.trim().to_string())
        .filter(|token| !token.is_empty())
}

/// No session for this origin — and the command that opens one.
#[derive(Debug)]
pub struct NotSignedIn {
    pub origin: String,
    pub login: String,
}

impl std::fmt::Display for NotSignedIn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&crate::tr!(
            "not signed in to {} — run `{}`",
            "aucune session sur {} — lancez `{}`",
            self.origin,
            self.login
        ))
    }
}

impl std::error::Error for NotSignedIn {}

/// Reads the access token that will be sent to `destination`: environment first, then the
/// session stored for that origin.
///
/// The environment wins so CI can inject a token without a keychain — a build agent has none.
///
/// Sessions are stored per origin: a stored session only goes back to the platform that issued
/// it. `--api`, `--env` and `PORTAKI_API_URL` choose where a command talks, and a `.envrc` in
/// someone else's repository sets the last: without this, cloning it was enough to hand them the
/// session. Signing in to staging leaves the production session alone.
pub fn access_token(destination: &str) -> Result<String> {
    ensure_transport(destination)?;
    if let Some(token) = explicit_token() {
        return Ok(token);
    }
    let origin = origin_of(destination).with_context(|| format!("{destination} is not a URL"))?;
    match read(ACCESS_ENTRY, &origin)? {
        Some(token) => Ok(token),
        None => Err(NotSignedIn {
            login: crate::profile::login_command(destination),
            origin,
        }
        .into()),
    }
}

/// `https`, or plain `http` to this machine only: a token sent in clear crosses the network.
pub fn ensure_transport(url: &str) -> Result<()> {
    if secure_or_loopback(url) {
        return Ok(());
    }
    bail!("refusing to send credentials to {url} — https is required outside localhost")
}

/// `https://…`, or `http://` to localhost / a loopback address.
pub fn secure_or_loopback(url: &str) -> bool {
    let Ok(parsed) = reqwest::Url::parse(url) else {
        return false;
    };
    match parsed.scheme() {
        "https" => true,
        "http" => {
            let host = parsed.host_str().unwrap_or_default();
            let host = host.trim_start_matches('[').trim_end_matches(']');
            host.eq_ignore_ascii_case("localhost")
                || host
                    .parse::<std::net::IpAddr>()
                    .is_ok_and(|ip| ip.is_loopback())
        }
        _ => false,
    }
}

/// `scheme://host[:port]`, the part that says who receives a request.
pub fn origin_of(url: &str) -> Option<String> {
    reqwest::Url::parse(url)
        .ok()
        .map(|parsed| parsed.origin().ascii_serialization())
        .filter(|origin| origin != "null")
}

/// Renews the access token and stores the rotated pair.
///
/// The access token lives fifteen minutes, a `--watch` session far longer. Without this, that
/// session would stop halfway through on a 401, and the only way out would be `portaki login`.
///
/// The platform now remembers the client and the scopes attached to the refresh token, so the
/// renewed token opens the same doors as the first one — without that memory, it came back with
/// the `portaki-api` audience and nothing else.
///
/// # One renewal at a time
///
/// Every renewal revokes the refresh token that was presented, and the platform takes the
/// presentation of an already-rotated token for a theft: it then revokes every session of the
/// account. Two `portaki` started together — a `dev --watch` and an `sdk upgrade`, two
/// worktrees — expire in the same minute and renew together: the second was presenting the token
/// the first had just rotated, and everybody ended up signed out.
///
/// Hence the lock, then the re-read: `stale` is the token that has just taken the 401. If the
/// stored token is no longer that one, another process renewed while we were waiting, and its
/// pair is ours too.
///
/// `auth_url` is the platform of the command in progress: it is the session of its origin that
/// gets renewed, never another origin's.
pub async fn refresh(auth_url: &str, stale: &str) -> Result<String> {
    ensure_transport(auth_url)?;
    let origin = origin_of(auth_url).with_context(|| format!("{auth_url} is not a URL"))?;
    let _held = RefreshLock::acquire(&config_dir()?.join(REFRESH_LOCK)).await?;

    if let Some(current) = read(ACCESS_ENTRY, &origin)? {
        if current != stale {
            return Ok(current);
        }
    }

    let refresh_token = match read(REFRESH_ENTRY, &origin)? {
        Some(token) => token,
        None => bail!(
            "no refresh token stored for {origin} — run `{}`",
            crate::profile::login_command(auth_url)
        ),
    };

    let response = crate::http::client()
        .post(format!("{auth_url}/api/v1/auth/refresh"))
        .json(&serde_json::json!({ "refreshToken": refresh_token }))
        .send()
        .await
        .context("renew the access token")?;
    let body = response.text().await.unwrap_or_default();
    let renewed: RenewedTokens = crate::api::unwrap(&body)?;

    // Rotation invalidates the old refresh token: not storing the new one would amount to
    // signing ourselves out at the next renewal.
    store(&origin, &renewed.access_token, &renewed.refresh_token)?;
    Ok(renewed.access_token)
}

const REFRESH_LOCK: &str = "refresh.lock";

/// Past this, the holder died without giving the lock back. Longer than a request's deadline
/// ([`crate::http::REQUEST`]): a renewal that is still alive never lasts that long.
const ABANDONED: Duration = Duration::from_secs(20);

/// Longer than [`ABANDONED`]: a lock left behind by a killed process frees itself during the
/// wait, instead of failing whoever is waiting.
const LOCK_WAIT: Duration = Duration::from_secs(30);

/// A file created exclusively, and not `File::lock`: that one requires Rust 1.89, beyond the
/// declared minimum version.
struct RefreshLock(PathBuf);

impl RefreshLock {
    async fn acquire(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create {}", parent.display()))?;
        }
        let deadline = Instant::now() + LOCK_WAIT;
        loop {
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
            {
                Ok(_) => return Ok(Self(path.to_path_buf())),
                Err(taken) if taken.kind() == std::io::ErrorKind::AlreadyExists => {
                    // ponytail: two processes can declare the same lock abandoned and take it
                    // over together — that takes a crash and then two renewals in the same
                    // second; a `File::lock` will settle it once the MSRV allows for one.
                    if is_abandoned(path) {
                        let _ = std::fs::remove_file(path);
                        continue;
                    }
                    if Instant::now() >= deadline {
                        bail!("another portaki process is renewing the session — try again");
                    }
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                Err(failure) => {
                    return Err(failure).with_context(|| format!("create {}", path.display()))
                }
            }
        }
    }
}

impl Drop for RefreshLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn is_abandoned(path: &Path) -> bool {
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|modified| modified.elapsed().ok())
        .is_some_and(|age| age > ABANDONED)
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RenewedTokens {
    access_token: String,
    refresh_token: String,
}

fn store(origin: &str, access_token: &str, refresh_token: &str) -> Result<()> {
    write(ACCESS_ENTRY, origin, access_token)?;
    write(REFRESH_ENTRY, origin, refresh_token)
}

/// Stores the session the platform `issuer` has just issued: that is the only place it will go
/// back to, and it touches no other origin's session.
pub fn store_issued_by(issuer: &str, access_token: &str, refresh_token: &str) -> Result<()> {
    let origin = origin_of(issuer).with_context(|| format!("{issuer} is not a URL"))?;
    store(&origin, access_token, refresh_token)
}

/// Where the session lives, as a person would look for it.
pub fn storage_label() -> String {
    if uses_keychain() {
        return "the system keychain".to_string();
    }
    credentials_path()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|_| "the credentials file".to_string())
}

/// The refresh token stored for this origin, if there is one.
///
/// Returned so that `logout` can present it to the server: erasing it from here does not revoke
/// it, and a session you believe closed would stay open until it expires.
pub fn refresh_token(origin: &str) -> Option<String> {
    read(REFRESH_ENTRY, origin).ok().flatten()
}

/// Forgets this origin's session — and only that one.
pub fn forget(origin: &str) -> Result<()> {
    if !uses_keychain() {
        let mut stored = load()?;
        stored.sessions.remove(origin);
        return if stored.sessions.is_empty() {
            remove_file(&credentials_path()?)
        } else {
            save(&stored)
        };
    }
    delete(ACCESS_ENTRY, origin)?;
    delete(REFRESH_ENTRY, origin)
}

/// The origins where a session is stored — so we can say "you are signed in elsewhere".
pub fn signed_in_origins() -> Vec<String> {
    if uses_keychain() {
        // The keychain cannot be enumerated: we can only answer for a given origin.
        return Vec::new();
    }
    load()
        .map(|stored| stored.sessions.into_keys().collect())
        .unwrap_or_default()
}

/// The keychain stays available for whoever prefers it.
fn uses_keychain() -> bool {
    std::env::var("PORTAKI_CREDENTIALS")
        .map(|choice| choice.trim().eq_ignore_ascii_case("keychain"))
        .unwrap_or(false)
}

/// `$XDG_CONFIG_HOME/portaki/credentials.json`, or `~/.config/…` failing that.
///
/// Outside the repository, always: a secrets file inside a working tree ends up being committed,
/// or swept up by a `git add -A`.
fn credentials_path() -> Result<PathBuf> {
    if let Ok(explicit) = std::env::var("PORTAKI_CREDENTIALS_FILE") {
        if !explicit.trim().is_empty() {
            return Ok(PathBuf::from(explicit));
        }
    }
    Ok(config_dir()?.join("credentials.json"))
}

/// The directory where the CLI stores what belongs to this person on this machine.
///
/// Outside the repository, always: what lives here travels across projects, and has no business
/// in a working tree.
pub fn config_dir() -> Result<PathBuf> {
    let base = match std::env::var("XDG_CONFIG_HOME") {
        Ok(xdg) if !xdg.trim().is_empty() => PathBuf::from(xdg),
        _ => {
            let home = std::env::var("HOME").context("locate the home directory")?;
            PathBuf::from(home).join(".config")
        }
    };
    Ok(base.join("portaki"))
}

/// A session: the pair of tokens one origin issued.
#[derive(Default, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Session {
    #[serde(default)]
    access_token: String,
    #[serde(default)]
    refresh_token: String,
}

/// The file: one session per origin.
///
/// The flat fields are the older ones — a single session, and its origin. They are read back as
/// the session of that origin, and they are never written again.
#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredCredentials {
    #[serde(default)]
    sessions: std::collections::BTreeMap<String, Session>,
    #[serde(default, skip_serializing)]
    access_token: String,
    #[serde(default, skip_serializing)]
    refresh_token: String,
    #[serde(default, skip_serializing)]
    origin: String,
}

impl StoredCredentials {
    /// Folds the old shape into the new one.
    fn migrated(mut self) -> Self {
        if !self.access_token.trim().is_empty() {
            let origin = if self.origin.trim().is_empty() {
                LEGACY_ORIGIN.to_string()
            } else {
                self.origin.clone()
            };
            self.sessions.entry(origin).or_insert(Session {
                access_token: std::mem::take(&mut self.access_token),
                refresh_token: std::mem::take(&mut self.refresh_token),
            });
        }
        self
    }
}

fn load() -> Result<StoredCredentials> {
    load_from(&credentials_path()?)
}

/// The path is a parameter so that testing this does not depend on the process environment —
/// shared by every test, and therefore a source of genuine intermittent failures.
fn load_from(path: &std::path::Path) -> Result<StoredCredentials> {
    match std::fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str::<StoredCredentials>(&raw)
            .map(StoredCredentials::migrated)
            .with_context(|| {
                format!(
                    "parse {} — delete it and run `portaki login`",
                    path.display()
                )
            }),
        Err(missing) if missing.kind() == std::io::ErrorKind::NotFound => {
            Ok(StoredCredentials::default())
        }
        Err(failure) => Err(failure).with_context(|| format!("read {}", path.display())),
    }
}

/// Written by rename: an interruption never leaves a truncated secrets file, which would force a
/// fresh sign-in for an entirely unrelated reason.
fn save(credentials: &StoredCredentials) -> Result<()> {
    save_to(&credentials_path()?, credentials)
}

fn save_to(path: &std::path::Path, credentials: &StoredCredentials) -> Result<()> {
    let parent = path.parent().context("credentials directory")?;
    std::fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    restrict(parent, 0o700)?;

    let temporary = path.with_extension("json.tmp");
    let body = serde_json::to_string_pretty(credentials).context("serialise credentials")?;
    // Created as 0600, not written and then restricted: in between, the file existed as 0644. A
    // leftover from an interrupted write is removed first, its permissions are not ours.
    let _ = std::fs::remove_file(&temporary);
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    std::io::Write::write_all(
        &mut options
            .open(&temporary)
            .with_context(|| format!("write {}", temporary.display()))?,
        body.as_bytes(),
    )
    .with_context(|| format!("write {}", temporary.display()))?;
    std::fs::rename(&temporary, path).with_context(|| format!("write {}", path.display()))
}

fn remove_file(path: &Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(missing) if missing.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(failure) => Err(failure).with_context(|| format!("remove {}", path.display())),
    }
}

#[cfg(unix)]
fn restrict(path: &std::path::Path, mode: u32) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
        .with_context(|| format!("restrict {}", path.display()))
}

/// Elsewhere, the default ACLs of a user profile do the job.
#[cfg(not(unix))]
fn restrict(_path: &std::path::Path, _mode: u32) -> Result<()> {
    Ok(())
}

fn entry(name: &str) -> Result<keyring::Entry> {
    keyring::Entry::new(SERVICE, name).context("open the system keychain")
}

/// `access-token@https://api.portaki.app`: one keychain entry per origin.
fn keyed(name: &str, origin: &str) -> String {
    format!("{name}@{origin}")
}

fn keychain_get(name: &str) -> Result<Option<String>> {
    match entry(name)?.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(failure) => Err(failure).context("read from the system keychain"),
    }
}

fn read(name: &str, origin: &str) -> Result<Option<String>> {
    if !uses_keychain() {
        let stored = load()?;
        let value = stored.sessions.get(origin).map(|session| match name {
            ACCESS_ENTRY => session.access_token.clone(),
            _ => session.refresh_token.clone(),
        });
        return Ok(value.filter(|value| !value.trim().is_empty()));
    }
    if let Some(value) = keychain_get(&keyed(name, origin))? {
        return Ok(Some(value));
    }
    // An older entry, with no origin in its name: it counts for the origin stored alongside it.
    let legacy = keychain_get(ORIGIN_ENTRY)?.unwrap_or_else(|| LEGACY_ORIGIN.to_string());
    if legacy == origin {
        return keychain_get(name);
    }
    Ok(None)
}

fn write(name: &str, origin: &str, value: &str) -> Result<()> {
    if !uses_keychain() {
        let mut stored = load()?;
        let session = stored.sessions.entry(origin.to_string()).or_default();
        match name {
            ACCESS_ENTRY => session.access_token = value.to_string(),
            _ => session.refresh_token = value.to_string(),
        }
        return save(&stored);
    }
    entry(&keyed(name, origin))?
        .set_password(value)
        .context("write to the system keychain")
}

fn delete(name: &str, origin: &str) -> Result<()> {
    let legacy = keychain_get(ORIGIN_ENTRY)?.unwrap_or_else(|| LEGACY_ORIGIN.to_string());
    let mut names = vec![keyed(name, origin)];
    if legacy == origin {
        names.push(name.to_string());
    }
    for name in names {
        match entry(&name)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => {}
            Err(failure) => return Err(failure).context("clear the system keychain"),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROD: &str = "https://api.portaki.app";

    /// A CI action that passes an optional input left unfilled exports an empty variable. Read
    /// as a URL, every call went off to `/registry/v1/...` — which reqwest refuses to build,
    /// with a "builder error" that points at nothing.
    #[tokio::test]
    async fn a_second_refresh_waits_for_the_first() {
        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory.path().join(REFRESH_LOCK);

        let first = RefreshLock::acquire(&path).await.expect("first");
        let second = tokio::spawn({
            let path = path.clone();
            async move { RefreshLock::acquire(&path).await }
        });
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert!(!second.is_finished(), "the second must wait");

        drop(first);
        let second = tokio::time::timeout(Duration::from_secs(2), second)
            .await
            .expect("released")
            .expect("join");
        assert!(second.is_ok());
    }

    #[tokio::test]
    async fn a_lock_left_by_a_dead_process_is_taken_back() {
        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory.path().join(REFRESH_LOCK);
        let left = std::fs::File::create(&path).expect("left behind");
        left.set_modified(std::time::SystemTime::now() - ABANDONED * 2)
            .expect("age it");

        tokio::time::timeout(Duration::from_secs(1), RefreshLock::acquire(&path))
            .await
            .expect("no wait")
            .expect("taken back");
    }

    /// Storage, put to the test without touching the process environment.
    ///
    /// The paths are passed as parameters: two tests that are configured through an environment
    /// variable run in parallel inside the same process and overwrite each other, which produces
    /// failures that have nothing to do with the code.
    #[test]
    fn credentials_round_trip_through_the_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("portaki").join("credentials.json");

        // Nothing stored: that is not a breakdown, that is "not signed in".
        let empty = load_from(&path).unwrap();
        assert!(empty.sessions.is_empty());

        let mut credentials = StoredCredentials::default();
        credentials.sessions.insert(
            PROD.into(),
            Session {
                access_token: "acces".into(),
                refresh_token: "renouvellement".into(),
            },
        );
        save_to(&path, &credentials).unwrap();

        let stored = load_from(&path).unwrap();
        assert_eq!(stored.sessions[PROD].access_token, "acces");
        assert_eq!(stored.sessions[PROD].refresh_token, "renouvellement");

        // The file is readable by its owner only — on a single-user machine that is the one
        // real protection, so it is the one that has to be checked.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "le fichier de secrets doit être en 0600");
            let parent = std::fs::metadata(path.parent().unwrap()).unwrap();
            assert_eq!(parent.permissions().mode() & 0o777, 0o700);
        }
    }

    /// The older file — one session, and its origin — is read back as the session of that
    /// origin, and of no other.
    #[test]
    fn a_legacy_file_is_read_as_the_session_of_its_origin() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("credentials.json");
        std::fs::write(
            &path,
            r#"{"accessToken":"a","refreshToken":"r","origin":"https://api-staging.portaki.app"}"#,
        )
        .unwrap();

        let stored = load_from(&path).unwrap();
        assert_eq!(
            stored.sessions["https://api-staging.portaki.app"].access_token,
            "a"
        );
        assert!(!stored.sessions.contains_key(PROD));

        // Rewritten, it keeps only the per-origin shape.
        save_to(&path, &stored).unwrap();
        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(
            raw.contains("sessions") && !raw.contains("\"origin\""),
            "{raw}"
        );
    }

    /// With no origin, the old session was production's.
    #[test]
    fn a_legacy_session_without_origin_is_production() {
        let stored =
            serde_json::from_str::<StoredCredentials>(r#"{"accessToken":"a","refreshToken":"r"}"#)
                .unwrap()
                .migrated();
        assert_eq!(stored.sessions[PROD].refresh_token, "r");
    }

    /// An `.envrc` that points `PORTAKI_API_URL` elsewhere does not receive the session: we look
    /// up by exact origin, scheme and port included.
    #[test]
    fn a_session_only_goes_back_to_its_origin() {
        assert_eq!(
            origin_of("https://api.portaki.app/registry/v1/x").as_deref(),
            Some(PROD)
        );
        for elsewhere in [
            "https://evil.example",
            "http://api.portaki.app",
            "https://api.portaki.app:8443",
            "https://api.portaki.app.evil.example",
        ] {
            assert_ne!(origin_of(elsewhere).as_deref(), Some(PROD), "{elsewhere}");
        }
        assert!(origin_of("not a url").is_none());
    }

    #[test]
    fn credentials_travel_over_https_or_to_this_machine_only() {
        for fine in [
            "https://api.portaki.app",
            "http://localhost:8080",
            "http://127.0.0.1:8080",
            "http://[::1]:8080",
        ] {
            assert!(secure_or_loopback(fine), "{fine}");
        }
        for refused in [
            "http://api.portaki.app",
            "http://localhost.evil.example",
            "file:///etc/passwd",
            "javascript:alert(1)",
            "",
        ] {
            assert!(!secure_or_loopback(refused), "{refused}");
        }
    }

    /// An unreadable file has to say so, it must not be left to be guessed at: the message has
    /// to name the way out, otherwise you go looking for a network failure.
    #[test]
    fn a_corrupt_file_says_what_to_do() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("credentials.json");
        std::fs::write(&path, "{ pas du json").unwrap();

        // `unwrap_err` would require `Debug` on `StoredCredentials`, and so a printable token in
        // a panic message or a backtrace. We read the error without asking for that.
        let failure = match load_from(&path) {
            Ok(_) => panic!("un fichier illisible ne doit pas passer pour vide"),
            Err(failure) => failure.to_string(),
        };
        assert!(failure.contains("portaki login"), "{failure}");
    }

    /// The default path lives outside the repository: a secrets file inside a working tree ends
    /// up being committed, or swept up by a `git add -A`.
    #[test]
    fn the_default_path_is_outside_any_repository() {
        let resolved = credentials_path().unwrap();

        assert!(
            resolved.ends_with("portaki/credentials.json"),
            "{resolved:?}"
        );
        assert!(resolved.is_absolute(), "{resolved:?}");
    }

    /// A single test for both cases: they share an environment variable, and splitting them
    /// would make them run in parallel inside the same process — and so overwrite each other at
    /// the whim of the scheduler.
    #[test]
    fn the_environment_is_the_way_in_when_there_is_no_keychain() {
        // A CI agent has no keychain: injection has to stay a way in.
        std::env::set_var("PORTAKI_DEV_TOKEN", "injected");
        assert_eq!(access_token(PROD).unwrap(), "injected");

        // An empty variable is not a token — otherwise we set off with a blank string.
        //
        // We do not require a failure: on a machine where `portaki login` has been run, the
        // keychain answers, and that is the intended behaviour. This test used to assert the
        // opposite and turned red from the very first sign-in — a test whose result depends on
        // the machine's history guards nothing. The real invariant is that a blank variable
        // never becomes a token.
        std::env::set_var("PORTAKI_DEV_TOKEN", "   ");
        match access_token(PROD) {
            Ok(from_keychain) => assert!(
                !from_keychain.trim().is_empty(),
                "une variable blanche ne doit pas devenir un jeton"
            ),
            Err(failure) => {
                let message = failure.to_string();
                assert!(
                    message.contains("portaki login") || message.contains("keychain"),
                    "{message}"
                );
            }
        }

        std::env::remove_var("PORTAKI_DEV_TOKEN");
    }
}
