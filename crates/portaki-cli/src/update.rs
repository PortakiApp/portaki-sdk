//! Prévenir qu'une version plus récente existe, sans jamais se mettre en travers.
//!
//! # Ce qui rendrait cet avis nuisible
//!
//! Un appel réseau à chaque commande. `portaki build` deviendrait plus lent parce qu'un jour
//! quelqu'un pourrait vouloir savoir qu'une version est sortie — l'inverse du service rendu.
//! La réponse est donc mise en cache un jour entier, et la seule commande qui la rafraîchit
//! abandonne au bout d'une seconde et demie.
//!
//! # Où il ne s'affiche pas
//!
//! Sous `--plain` : cette sortie est faite pour être lue par un programme, et une ligne de plus
//! y est un champ de plus à filtrer. Hors terminal non plus — un journal de CI n'a personne
//! pour agir dessus, et `portaki ci check` y dit déjà ce qui vieillit. Et jamais si
//! `PORTAKI_NO_UPDATE_CHECK` est posé.
//!
//! L'avis vient **après** la commande, une fois son travail rendu : il ne retarde rien de ce
//! qu'on attendait, et n'éloigne pas du regard la ligne qu'on est venu lire.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::ui;

/// La version publiée reste valable un jour : au-delà on redemande, en deçà on se tait.
const FRESH_FOR: Duration = Duration::from_secs(24 * 60 * 60);

/// Au-delà, la question ne vaut plus le temps qu'elle prend.
const GIVE_UP_AFTER: Duration = Duration::from_millis(1_500);

/// La variable qui éteint tout, pour qui ne veut pas en entendre parler.
const OPT_OUT: &str = "PORTAKI_NO_UPDATE_CHECK";

/// Dit qu'une version plus récente existe, si c'est le cas et si quelqu'un est là pour le lire.
pub async fn notify() {
    if !wanted() {
        return;
    }
    let Some(latest) = latest().await else {
        return;
    };
    let running = env!("CARGO_PKG_VERSION");
    if !outdated(running, &latest) {
        return;
    }

    ui::blank();
    ui::warn(format!("portaki {running} → {latest} is available"));
    ui::detail("cargo install portaki-cli --locked --force");
    ui::detail(format!("{OPT_OUT}=1 silences this"));
}

/// Y a-t-il quelqu'un pour lire, et le veut-il ?
fn wanted() -> bool {
    !ui::plain() && console::user_attended() && std::env::var_os(OPT_OUT).is_none()
}

/// La dernière version publiée, du cache tant qu'il est frais.
async fn latest() -> Option<String> {
    if let Some(cached) = read_cache() {
        return Some(cached);
    }
    let fetched = tokio::time::timeout(GIVE_UP_AFTER, ask_crates_io())
        .await
        .ok()?
        .ok()?;
    write_cache(&fetched);
    Some(fetched)
}

async fn ask_crates_io() -> Result<String, reqwest::Error> {
    let body: serde_json::Value = reqwest::Client::new()
        .get("https://crates.io/api/v1/crates/portaki-cli")
        // crates.io refuse une requête sans agent identifiable, et le dit en 403.
        .header(
            "User-Agent",
            concat!("portaki-cli/", env!("CARGO_PKG_VERSION")),
        )
        .send()
        .await?
        .json()
        .await?;
    Ok(body
        .pointer("/crate/max_stable_version")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string())
}

fn read_cache() -> Option<String> {
    parse_cache(&std::fs::read_to_string(cache_path()?).ok()?, now())
}

/// `<horodatage> <version>` — deux champs, une ligne, aucun format à faire évoluer.
///
/// Séparé de la lecture du fichier pour être vérifiable : un cache abîmé doit se lire comme
/// une absence, jamais comme une version, sinon un fichier tronqué ferait annoncer n'importe
/// quoi comme la dernière version publiée.
fn parse_cache(raw: &str, now: Duration) -> Option<String> {
    let (stamped, version) = raw.trim().split_once(' ')?;
    let stamped = Duration::from_secs(stamped.parse().ok()?);
    if version.is_empty() || now.checked_sub(stamped)? > FRESH_FOR {
        return None;
    }
    Some(version.to_string())
}

fn write_cache(version: &str) {
    let Some(path) = cache_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    // Un cache qui ne s'écrit pas ne casse rien : on redemandera, voilà tout.
    let _ = std::fs::write(&path, format!("{} {version}", now().as_secs()));
}

fn cache_path() -> Option<std::path::PathBuf> {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| std::path::PathBuf::from(home).join(".cache"))
        })?;
    Some(base.join("portaki").join("latest-version"))
}

fn now() -> Duration {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
}

/// `running` est-il en retard sur `latest` ?
///
/// Comparé composant par composant, en nombres : `2.10.0` est postérieur à `2.9.0`, ce qu'un
/// ordre lexicographique inverserait.
pub fn outdated(running: &str, latest: &str) -> bool {
    parts(latest) > parts(running)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare_as_numbers_not_as_text() {
        assert!(outdated("2.9.0", "2.10.0"));
        assert!(!outdated("2.10.0", "2.9.0"));
        assert!(!outdated("2.4.0", "2.4.0"));
    }

    /// Une préversion n'est pas une version plus récente : `2.5.0-rc.1` ne doit pas pousser
    /// quelqu'un qui tourne en `2.5.0` à « mettre à jour » vers ce qu'il dépasse déjà.
    #[test]
    fn a_prerelease_suffix_is_ignored() {
        assert!(!outdated("2.5.0", "2.5.0-rc.1"));
    }

    const NOW: Duration = Duration::from_secs(1_000_000);

    #[test]
    fn a_fresh_cache_answers() {
        let written = NOW.as_secs() - 60;

        assert_eq!(
            parse_cache(&format!("{written} 2.4.0"), NOW).as_deref(),
            Some("2.4.0")
        );
    }

    /// Passé un jour, on redemande — sans quoi une version publiée resterait invisible.
    #[test]
    fn a_stale_cache_asks_again() {
        let written = NOW.as_secs() - FRESH_FOR.as_secs() - 1;

        assert!(parse_cache(&format!("{written} 2.4.0"), NOW).is_none());
    }

    /// Un cache abîmé se lit comme une absence, jamais comme une version : un fichier tronqué
    /// ferait sinon annoncer n'importe quoi comme la dernière version publiée.
    #[test]
    fn a_damaged_cache_reads_as_no_answer() {
        for raw in ["", "   ", "n importe quoi", "pas-un-nombre 2.4.0", "1000 "] {
            assert!(parse_cache(raw, NOW).is_none(), "accepté : {raw:?}");
        }
    }

    /// Une horloge qui recule — correction NTP, machine réveillée — écrit un horodatage dans
    /// le futur. On redemande alors, plutôt que de faire confiance à une fraîcheur qu'on ne
    /// sait pas calculer : redemander ne coûte qu'une requête, s'en remettre à un cache qu'on
    /// ne comprend pas pourrait taire l'avis très longtemps.
    #[test]
    fn a_cache_written_in_the_future_is_asked_again() {
        let written = NOW.as_secs() + 10;

        assert!(parse_cache(&format!("{written} 2.4.0"), NOW).is_none());
    }
}
