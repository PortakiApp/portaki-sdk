//! Where the CLI keeps its credentials.
//!
//! In the system keychain — Keychain on macOS, Secret Service on Linux, Credential Manager on
//! Windows — and not in a plain file under `~/.portaki/`. Three lines more, and a stray `cat`
//! during a screencast no longer broadcasts a week of access.

use anyhow::{bail, Context, Result};

const SERVICE: &str = "app.portaki.cli";
const ACCESS_ENTRY: &str = "access-token";
const REFRESH_ENTRY: &str = "refresh-token";

/// Reads the access token: environment first, then the keychain.
///
/// The environment wins so CI can inject a token without a keychain — a build agent has none.
pub fn access_token() -> Result<String> {
    if let Ok(token) = std::env::var("PORTAKI_DEV_TOKEN") {
        if !token.trim().is_empty() {
            return Ok(token);
        }
    }
    match read(ACCESS_ENTRY) {
        Ok(Some(token)) => Ok(token),
        Ok(None) => bail!("not signed in — run `portaki login`"),
        Err(failure) => Err(failure),
    }
}

pub fn store(access_token: &str, refresh_token: &str) -> Result<()> {
    write(ACCESS_ENTRY, access_token)?;
    write(REFRESH_ENTRY, refresh_token)
}

pub fn forget() -> Result<()> {
    delete(ACCESS_ENTRY)?;
    delete(REFRESH_ENTRY)
}

fn entry(name: &str) -> Result<keyring::Entry> {
    keyring::Entry::new(SERVICE, name).context("open the system keychain")
}

fn read(name: &str) -> Result<Option<String>> {
    match entry(name)?.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(failure) => Err(failure).context("read from the system keychain"),
    }
}

fn write(name: &str, value: &str) -> Result<()> {
    entry(name)?
        .set_password(value)
        .context("write to the system keychain")
}

fn delete(name: &str) -> Result<()> {
    match entry(name)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(failure) => Err(failure).context("clear the system keychain"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un seul test pour les deux cas : ils partagent une variable d'environnement, et les
    /// séparer les ferait courir en parallèle dans le même processus — donc s'écraser l'un
    /// l'autre au hasard de l'ordonnancement.
    #[test]
    fn the_environment_is_the_way_in_when_there_is_no_keychain() {
        // Un agent de CI n'a pas de trousseau : l'injection doit rester une porte d'entrée.
        std::env::set_var("PORTAKI_DEV_TOKEN", "injected");
        assert_eq!(access_token().unwrap(), "injected");

        // Une variable vide n'est pas un jeton — sinon on part avec une chaîne blanche.
        std::env::set_var("PORTAKI_DEV_TOKEN", "   ");
        let failure = access_token().unwrap_err().to_string();
        assert!(
            failure.contains("portaki login") || failure.contains("keychain"),
            "{failure}"
        );

        std::env::remove_var("PORTAKI_DEV_TOKEN");
    }
}
