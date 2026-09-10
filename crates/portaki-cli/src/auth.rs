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

use std::path::PathBuf;

use anyhow::{bail, Context, Result};

const SERVICE: &str = "app.portaki.cli";
const ACCESS_ENTRY: &str = "access-token";
const REFRESH_ENTRY: &str = "refresh-token";

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

/// Reads the access token: environment first, then the keychain.
///
/// The environment wins so CI can inject a token without a keychain — a build agent has none.
pub fn access_token() -> Result<String> {
    if let Some(token) = explicit_token() {
        return Ok(token);
    }
    match read(ACCESS_ENTRY) {
        Ok(Some(token)) => Ok(token),
        Ok(None) => bail!("not signed in — run `portaki login`"),
        Err(failure) => Err(failure),
    }
}

/// Renouvelle le jeton d'accès et range la paire tournée.
///
/// Le jeton d'accès vit quinze minutes, une session `--watch` bien plus. Sans ceci, elle
/// s'arrêterait au milieu sur un 401, et la seule issue serait de relancer `portaki login`.
///
/// La plateforme se souvient désormais du client et des scopes attachés au jeton de
/// rafraîchissement, donc le jeton renouvelé ouvre les mêmes portes que le premier — sans cette
/// mémoire, il repartait avec la seule audience `portaki-api`.
pub async fn refresh() -> Result<String> {
    let refresh_token = match read(REFRESH_ENTRY)? {
        Some(token) => token,
        None => bail!("no refresh token stored — run `portaki login`"),
    };

    let response = reqwest::Client::new()
        .post(format!("{}/api/v1/auth/refresh", api_base_url(None)))
        .json(&serde_json::json!({ "refreshToken": refresh_token }))
        .send()
        .await
        .context("renew the access token")?;
    let body = response.text().await.unwrap_or_default();
    let renewed: RenewedTokens = crate::api::unwrap(&body)?;

    // La rotation invalide l'ancien jeton de rafraîchissement : ne pas ranger le nouveau
    // reviendrait à se déconnecter au renouvellement suivant.
    store(&renewed.access_token, &renewed.refresh_token)?;
    Ok(renewed.access_token)
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RenewedTokens {
    access_token: String,
    refresh_token: String,
}

/// `--url`, puis `PORTAKI_API_URL`, puis la production.
///
/// Une valeur vide ou blanche vaut « non définie », pas « URL vide ». `env::var` rend `Ok("")`
/// pour une variable exportée vide, et l'URL de base devenait alors la chaîne vide : chaque
/// appel partait vers `/registry/v1/...`, que reqwest refuse de construire. Une action de CI
/// qui passe une entrée facultative non renseignée exporte exactement ça.
pub fn api_base_url(explicit: Option<&str>) -> String {
    resolve_base_url(explicit, std::env::var("PORTAKI_API_URL").ok().as_deref())
}

/// La règle seule, sans l'environnement, pour être vérifiable.
fn resolve_base_url(explicit: Option<&str>, from_env: Option<&str>) -> String {
    [explicit, from_env]
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|value| !value.is_empty())
        .unwrap_or("https://api.portaki.app")
        .trim_end_matches('/')
        .to_string()
}

pub fn store(access_token: &str, refresh_token: &str) -> Result<()> {
    write(ACCESS_ENTRY, access_token)?;
    write(REFRESH_ENTRY, refresh_token)
}

/// Le jeton de rafraîchissement rangé, s'il y en a un.
///
/// Rendu pour que `logout` puisse le présenter au serveur : l'effacer d'ici ne le révoque pas,
/// et une session qu'on croit fermée resterait ouverte jusqu'à son expiration.
pub fn refresh_token() -> Option<String> {
    read(REFRESH_ENTRY).ok().flatten()
}

pub fn forget() -> Result<()> {
    delete(ACCESS_ENTRY)?;
    delete(REFRESH_ENTRY)
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

#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredCredentials {
    #[serde(default)]
    access_token: String,
    #[serde(default)]
    refresh_token: String,
}

fn load() -> Result<StoredCredentials> {
    load_from(&credentials_path()?)
}

/// Le chemin en paramètre, pour que l'éprouver ne dépende pas de l'environnement du processus —
/// partagé par tous les tests, donc source de vraies intermittences.
fn load_from(path: &std::path::Path) -> Result<StoredCredentials> {
    match std::fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str(&raw).with_context(|| {
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
    std::fs::write(&temporary, body).with_context(|| format!("write {}", temporary.display()))?;
    // Les droits AVANT le renommage : entre l'écriture et le chmod, le fichier existerait en
    // clair et lisible par tous.
    restrict(&temporary, 0o600)?;
    std::fs::rename(&temporary, path).with_context(|| format!("write {}", path.display()))
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

fn read(name: &str) -> Result<Option<String>> {
    if !uses_keychain() {
        let stored = load()?;
        let value = if name == ACCESS_ENTRY {
            stored.access_token
        } else {
            stored.refresh_token
        };
        return Ok(if value.trim().is_empty() {
            None
        } else {
            Some(value)
        });
    }
    match entry(name)?.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(failure) => Err(failure).context("read from the system keychain"),
    }
}

fn write(name: &str, value: &str) -> Result<()> {
    if !uses_keychain() {
        let mut stored = load()?;
        if name == ACCESS_ENTRY {
            stored.access_token = value.to_string();
        } else {
            stored.refresh_token = value.to_string();
        }
        return save(&stored);
    }
    entry(name)?
        .set_password(value)
        .context("write to the system keychain")
}

fn delete(name: &str) -> Result<()> {
    if !uses_keychain() {
        // Le fichier entier part au premier appel : il ne porte que ces deux jetons, et en
        // laisser un seul rendrait un `logout` à moitié fait.
        let path = credentials_path()?;
        return match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(missing) if missing.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(failure) => Err(failure).with_context(|| format!("remove {}", path.display())),
        };
    }
    match entry(name)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(failure) => Err(failure).context("clear the system keychain"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROD: &str = "https://api.portaki.app";

    /// Une action de CI qui passe une entrée facultative non renseignée exporte une variable
    /// vide. Lue comme une URL, chaque appel partait vers `/registry/v1/...` — que reqwest
    /// refuse de construire, avec un « builder error » qui ne désigne rien.
    #[test]
    fn an_exported_but_empty_variable_means_unset() {
        assert_eq!(resolve_base_url(None, Some("")), PROD);
        assert_eq!(resolve_base_url(None, Some("   ")), PROD);
        assert_eq!(resolve_base_url(Some(""), None), PROD);
    }

    #[test]
    fn the_flag_wins_over_the_environment() {
        assert_eq!(
            resolve_base_url(
                Some("https://explicit.example"),
                Some("https://env.example")
            ),
            "https://explicit.example"
        );
    }

    /// Une barre finale ne doit jamais doubler celle du chemin qu'on y accole.
    #[test]
    fn a_trailing_slash_never_doubles() {
        assert_eq!(
            resolve_base_url(None, Some("https://api.example/")),
            "https://api.example"
        );
    }

    #[test]
    fn nothing_set_means_production() {
        assert_eq!(resolve_base_url(None, None), PROD);
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
        assert!(empty.access_token.is_empty());

        save_to(
            &path,
            &StoredCredentials {
                access_token: "acces".into(),
                refresh_token: "renouvellement".into(),
            },
        )
        .unwrap();

        let stored = load_from(&path).unwrap();
        assert_eq!(stored.access_token, "acces");
        assert_eq!(stored.refresh_token, "renouvellement");

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
        assert_eq!(access_token().unwrap(), "injected");

        // Une variable vide n'est pas un jeton — sinon on part avec une chaîne blanche.
        //
        // On n'exige pas d'échec : sur une machine où `portaki login` est passé, le trousseau
        // répond, et c'est le comportement voulu. Ce test affirmait le contraire et devenait
        // rouge dès la première connexion — un test dont le résultat dépend de l'historique de
        // la machine ne garde rien. L'invariant réel est qu'une variable blanche ne devient
        // jamais un jeton.
        std::env::set_var("PORTAKI_DEV_TOKEN", "   ");
        match access_token() {
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
