//! Where the CLI keeps its credentials.
//!
//! Dans un fichier, `~/.config/portaki/credentials.json`, en `0600` — et non plus dans le
//! trousseau du système.
//!
//! Le trousseau était le bon choix sur le papier : chiffré au repos, verrouillé avec la session.
//! Il l'est resté jusqu'à ce qu'on constate son coût réel sur macOS — il attache son
//! autorisation à l'identité de code du binaire, et un binaire recompilé est un inconnu. Une
//! boucle de développement qui recompile redemande donc le mot de passe de session à chaque
//! passage. Un garde-fou qu'on affronte cent fois par jour finit par être contourné ; celui-ci
//! l'était déjà, par la variable d'environnement.
//!
//! Ce que le fichier garde :
//!
//! - `0600` sur le fichier, `0700` sur son dossier — sur une machine mono-utilisateur, c'est la
//!   protection qui compte réellement ;
//! - hors du dépôt, sous `$XDG_CONFIG_HOME`, donc jamais commité ni pris dans un `git add -A` ;
//! - écrit par renommage atomique : une interruption ne laisse pas un fichier tronqué ;
//! - jamais affiché, et `portaki logout` l'efface.
//!
//! Ce qu'il ne garde pas : le chiffrement au repos. **Hacher est impossible** — un jeton doit
//! être rejoué tel quel, et un condensat ne se rejoue pas. Chiffrer demanderait une clé, qu'il
//! faudrait ranger… dans le trousseau qu'on vient de quitter. Le dire vaut mieux que de brouiller
//! le contenu pour s'en donner l'air.
//!
//! `PORTAKI_CREDENTIALS=keychain` restaure l'ancien comportement, pour qui le préfère.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};

const SERVICE: &str = "app.portaki.cli";
const ACCESS_ENTRY: &str = "access-token";
const REFRESH_ENTRY: &str = "refresh-token";
/// L'origine d'une session rangée avant qu'elles soient rangées par origine.
const ORIGIN_ENTRY: &str = "origin";

/// Une session rangée avant qu'on retienne son origine : la production, la seule par défaut.
const LEGACY_ORIGIN: &str = "https://api.portaki.app";

/// Le jeton posé explicitement dans l'environnement, s'il y en a un.
///
/// Il gagne sur tout le reste, y compris sur l'OIDC d'une CI : un choix explicite doit primer
/// sur un mécanisme qui s'active tout seul, sans quoi poser cette variable n'aurait plus d'effet
/// visible et le débogage deviendrait un jeu de devinettes.
pub fn explicit_token() -> Option<String> {
    std::env::var("PORTAKI_DEV_TOKEN")
        .ok()
        .map(|token| token.trim().to_string())
        .filter(|token| !token.is_empty())
}

/// Pas de session pour cette origine — et la commande qui en ouvre une.
#[derive(Debug)]
pub struct NotSignedIn {
    pub origin: String,
    pub login: String,
}

impl std::fmt::Display for NotSignedIn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "not signed in to {} — run `{}`", self.origin, self.login)
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

/// Renouvelle le jeton d'accès et range la paire tournée.
///
/// Le jeton d'accès vit quinze minutes, une session `--watch` bien plus. Sans ceci, elle
/// s'arrêterait au milieu sur un 401, et la seule issue serait de relancer `portaki login`.
///
/// La plateforme se souvient désormais du client et des scopes attachés au jeton de
/// rafraîchissement, donc le jeton renouvelé ouvre les mêmes portes que le premier — sans cette
/// mémoire, il repartait avec la seule audience `portaki-api`.
///
/// # Un renouvellement à la fois
///
/// Chaque renouvellement révoque le jeton de rafraîchissement présenté, et la plateforme prend
/// la présentation d'un jeton déjà tourné pour un vol : elle révoque alors toutes les sessions
/// du compte. Deux `portaki` lancés ensemble — un `dev --watch` et un `sdk upgrade`, deux
/// worktrees — expirent à la même minute et renouvellent ensemble : le second présentait le
/// jeton que le premier venait de tourner, et tout le monde se retrouvait déconnecté.
///
/// D'où le verrou, puis la relecture : `stale` est le jeton qui vient d'essuyer le 401. Si le
/// jeton rangé n'est plus celui-là, un autre processus a renouvelé pendant qu'on attendait, et
/// sa paire est aussi la nôtre.
///
/// `auth_url` est la plateforme de la commande en cours : c'est la session de son origine qui
/// est renouvelée, jamais celle d'une autre.
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

    // La rotation invalide l'ancien jeton de rafraîchissement : ne pas ranger le nouveau
    // reviendrait à se déconnecter au renouvellement suivant.
    store(&origin, &renewed.access_token, &renewed.refresh_token)?;
    Ok(renewed.access_token)
}

const REFRESH_LOCK: &str = "refresh.lock";

/// Au-delà, le détenteur est mort sans rendre le verrou. Plus long que l'échéance d'une requête
/// ([`crate::http::REQUEST`]) : un renouvellement vivant ne dure jamais autant.
const ABANDONED: Duration = Duration::from_secs(20);

/// Plus long que [`ABANDONED`] : un verrou laissé par un processus tué se libère pendant
/// l'attente, au lieu de faire échouer celui qui attend.
const LOCK_WAIT: Duration = Duration::from_secs(30);

/// Un fichier créé en exclusif, et non `File::lock` : celui-ci demande Rust 1.89, au-delà de la
/// version minimale déclarée.
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
                    // ponytail: deux processus peuvent déclarer le même verrou abandonné et le
                    // reprendre ensemble — il faut un crash puis deux renouvellements à la
                    // même seconde ; un `File::lock` le réglera quand la MSRV le permettra.
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

/// Range la session que la plateforme `issuer` vient d'émettre : c'est la seule où elle
/// repartira, et elle ne touche à la session d'aucune autre origine.
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

/// Le jeton de rafraîchissement rangé pour cette origine, s'il y en a un.
///
/// Rendu pour que `logout` puisse le présenter au serveur : l'effacer d'ici ne le révoque pas,
/// et une session qu'on croit fermée resterait ouverte jusqu'à son expiration.
pub fn refresh_token(origin: &str) -> Option<String> {
    read(REFRESH_ENTRY, origin).ok().flatten()
}

/// Oublie la session de cette origine — et elle seule.
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

/// Les origines où une session est rangée — pour dire « tu es connecté ailleurs ».
pub fn signed_in_origins() -> Vec<String> {
    if uses_keychain() {
        // Le trousseau ne s'énumère pas : on ne sait répondre que pour une origine donnée.
        return Vec::new();
    }
    load()
        .map(|stored| stored.sessions.into_keys().collect())
        .unwrap_or_default()
}

/// Le trousseau reste accessible pour qui le préfère.
fn uses_keychain() -> bool {
    std::env::var("PORTAKI_CREDENTIALS")
        .map(|choice| choice.trim().eq_ignore_ascii_case("keychain"))
        .unwrap_or(false)
}

/// `$XDG_CONFIG_HOME/portaki/credentials.json`, ou `~/.config/…` à défaut.
///
/// Hors du dépôt, toujours : un fichier de secrets dans un arbre de travail finit par être
/// commité, ou balayé par un `git add -A`.
fn credentials_path() -> Result<PathBuf> {
    if let Ok(explicit) = std::env::var("PORTAKI_CREDENTIALS_FILE") {
        if !explicit.trim().is_empty() {
            return Ok(PathBuf::from(explicit));
        }
    }
    Ok(config_dir()?.join("credentials.json"))
}

/// Le dossier où la CLI range ce qui appartient à cette personne sur cette machine.
///
/// Hors du dépôt, toujours : ce qui vit ici traverse les projets, et n'a rien à faire dans un
/// arbre de travail.
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

/// Une session : la paire de jetons qu'une origine a émise.
#[derive(Default, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Session {
    #[serde(default)]
    access_token: String,
    #[serde(default)]
    refresh_token: String,
}

/// Le fichier : une session par origine.
///
/// Les champs à plat sont ceux d'avant — une seule session, et son origine. Ils sont relus
/// comme la session de cette origine, et ne sont plus jamais écrits.
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
    /// Replie l'ancienne forme dans la nouvelle.
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

/// Le chemin en paramètre, pour que l'éprouver ne dépende pas de l'environnement du processus —
/// partagé par tous les tests, donc source de vraies intermittences.
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

/// Écrit par renommage : une interruption ne laisse pas un fichier de secrets tronqué, ce qui
/// obligerait à se reconnecter pour une raison qui n'a rien à voir.
fn save(credentials: &StoredCredentials) -> Result<()> {
    save_to(&credentials_path()?, credentials)
}

fn save_to(path: &std::path::Path, credentials: &StoredCredentials) -> Result<()> {
    let parent = path.parent().context("credentials directory")?;
    std::fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    restrict(parent, 0o700)?;

    let temporary = path.with_extension("json.tmp");
    let body = serde_json::to_string_pretty(credentials).context("serialise credentials")?;
    // Créé en 0600, pas écrit puis restreint : entre les deux, le fichier existait en 0644. Un
    // reste d'une écriture interrompue est retiré d'abord, ses droits ne sont pas les nôtres.
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

/// Ailleurs, les ACL par défaut d'un profil utilisateur font le travail.
#[cfg(not(unix))]
fn restrict(_path: &std::path::Path, _mode: u32) -> Result<()> {
    Ok(())
}

fn entry(name: &str) -> Result<keyring::Entry> {
    keyring::Entry::new(SERVICE, name).context("open the system keychain")
}

/// `access-token@https://api.portaki.app` : une entrée de trousseau par origine.
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
    // Une entrée d'avant, sans origine dans son nom : elle vaut pour l'origine rangée à côté.
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

    /// Une action de CI qui passe une entrée facultative non renseignée exporte une variable
    /// vide. Lue comme une URL, chaque appel partait vers `/registry/v1/...` — que reqwest
    /// refuse de construire, avec un « builder error » qui ne désigne rien.
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

    /// Le stockage, éprouvé sans toucher l'environnement du processus.
    ///
    /// Les chemins sont passés en paramètre : deux tests qui se règlent par variable
    /// d'environnement courent en parallèle dans le même processus et s'écrasent l'un l'autre,
    /// ce qui produit des échecs qui n'ont rien à voir avec le code.
    #[test]
    fn credentials_round_trip_through_the_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("portaki").join("credentials.json");

        // Rien de stocké : ce n'est pas une panne, c'est « pas connecté ».
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

        // Le fichier n'est lisible que par son propriétaire — sur une machine
        // mono-utilisateur, c'est la seule protection réelle, donc celle qu'il faut vérifier.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "le fichier de secrets doit être en 0600");
            let parent = std::fs::metadata(path.parent().unwrap()).unwrap();
            assert_eq!(parent.permissions().mode() & 0o777, 0o700);
        }
    }

    /// Le fichier d'avant — une session, et son origine — se relit comme la session de cette
    /// origine, et d'aucune autre.
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

        // Réécrit, il ne garde que la forme par origine.
        save_to(&path, &stored).unwrap();
        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(
            raw.contains("sessions") && !raw.contains("\"origin\""),
            "{raw}"
        );
    }

    /// Sans origine, l'ancienne session était celle de la production.
    #[test]
    fn a_legacy_session_without_origin_is_production() {
        let stored =
            serde_json::from_str::<StoredCredentials>(r#"{"accessToken":"a","refreshToken":"r"}"#)
                .unwrap()
                .migrated();
        assert_eq!(stored.sessions[PROD].refresh_token, "r");
    }

    /// Un `.envrc` qui pointe `PORTAKI_API_URL` ailleurs ne reçoit pas la session : on cherche
    /// par origine exacte, schéma et port compris.
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

    /// Un fichier illisible se dit, il ne se devine pas : le message doit nommer la sortie,
    /// sinon on cherche une panne de réseau.
    #[test]
    fn a_corrupt_file_says_what_to_do() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("credentials.json");
        std::fs::write(&path, "{ pas du json").unwrap();

        // `unwrap_err` exigerait `Debug` sur `StoredCredentials`, donc un jeton imprimable dans
        // un message de panique ou une trace. On lit l'erreur sans le demander.
        let failure = match load_from(&path) {
            Ok(_) => panic!("un fichier illisible ne doit pas passer pour vide"),
            Err(failure) => failure.to_string(),
        };
        assert!(failure.contains("portaki login"), "{failure}");
    }

    /// Le chemin par défaut vit hors du dépôt : un fichier de secrets dans un arbre de travail
    /// finit par être commité, ou balayé par un `git add -A`.
    #[test]
    fn the_default_path_is_outside_any_repository() {
        let resolved = credentials_path().unwrap();

        assert!(
            resolved.ends_with("portaki/credentials.json"),
            "{resolved:?}"
        );
        assert!(resolved.is_absolute(), "{resolved:?}");
    }

    /// Un seul test pour les deux cas : ils partagent une variable d'environnement, et les
    /// séparer les ferait courir en parallèle dans le même processus — donc s'écraser l'un
    /// l'autre au hasard de l'ordonnancement.
    #[test]
    fn the_environment_is_the_way_in_when_there_is_no_keychain() {
        // Un agent de CI n'a pas de trousseau : l'injection doit rester une porte d'entrée.
        std::env::set_var("PORTAKI_DEV_TOKEN", "injected");
        assert_eq!(access_token(PROD).unwrap(), "injected");

        // Une variable vide n'est pas un jeton — sinon on part avec une chaîne blanche.
        //
        // On n'exige pas d'échec : sur une machine où `portaki login` est passé, le trousseau
        // répond, et c'est le comportement voulu. Ce test affirmait le contraire et devenait
        // rouge dès la première connexion — un test dont le résultat dépend de l'historique de
        // la machine ne garde rien. L'invariant réel est qu'une variable blanche ne devient
        // jamais un jeton.
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
