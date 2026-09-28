//! La langue de la CLI : celle du système, français ou anglais.
//!
//! `PORTAKI_LANG` d'abord, puis `LC_ALL`, `LC_MESSAGES` et `LANG` — l'ordre de POSIX. Une valeur
//! qui commence par `fr` donne le français, avec les mots de l'espace développeur ; toute autre,
//! l'anglais. Seul ce qu'une personne lit change : `--json` (toujours en anglais), les codes
//! d'erreur de la plateforme et les codes de sortie restent les mêmes dans les deux langues.
//!
//! Chaque phrase s'écrit à l'endroit où elle sert, dans les deux langues côte à côte
//! ([`tr!`](crate::tr)) : une phrase ajoutée sans sa traduction ne compile pas.

use std::sync::OnceLock;

/// Les variables lues, dans l'ordre où elles l'emportent.
const VARIABLES: [&str; 4] = ["PORTAKI_LANG", "LC_ALL", "LC_MESSAGES", "LANG"];

/// La CLI parle-t-elle français ?
///
/// Jamais sous `--json` : le document et les messages qu'il porte (`error`, `reason`) restent les
/// mêmes quelle que soit la machine, et ce qui part sur stderr avec lui aussi.
pub fn french() -> bool {
    static FRENCH: OnceLock<bool> = OnceLock::new();
    !crate::ui::json()
        && *FRENCH.get_or_init(|| {
            // Les tests unitaires ne suivent pas la machine : leurs phrases attendues sont anglaises.
            let variables: &[&str] = if cfg!(test) {
                &VARIABLES[..1]
            } else {
                &VARIABLES
            };
            is_french(variables.iter().map(|key| std::env::var(key).ok()))
        })
}

/// La première valeur non vide décide.
fn is_french(values: impl Iterator<Item = Option<String>>) -> bool {
    values
        .flatten()
        .find(|value| !value.trim().is_empty())
        .is_some_and(|value| value.trim().to_ascii_lowercase().starts_with("fr"))
}

/// La phrase dans la langue de la CLI : `tr!("english {x}", "français {x}", …)`.
#[macro_export]
macro_rules! tr {
    ($en:literal, $fr:literal $(, $arg:expr)* $(,)?) => {
        if $crate::lang::french() {
            format!($fr $(, $arg)*)
        } else {
            format!($en $(, $arg)*)
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decide(values: &[Option<&str>]) -> bool {
        is_french(values.iter().map(|value| value.map(str::to_string)))
    }

    #[test]
    fn the_first_variable_set_decides() {
        // PORTAKI_LANG, LC_ALL, LC_MESSAGES, LANG
        assert!(decide(&[None, None, None, Some("fr_FR.UTF-8")]));
        assert!(!decide(&[
            None,
            Some("en_US.UTF-8"),
            None,
            Some("fr_FR.UTF-8")
        ]));
        assert!(decide(&[Some("fr"), Some("en_US.UTF-8"), None, None]));
        assert!(!decide(&[Some("en"), None, None, Some("fr_FR.UTF-8")]));
        assert!(decide(&[None, None, Some("fr_CA"), Some("C")]));
    }

    /// Une variable posée vide ne décide rien : c'est la suivante qui parle.
    #[test]
    fn an_empty_variable_is_skipped() {
        assert!(decide(&[Some(""), Some(" "), None, Some("fr_BE.UTF-8")]));
    }

    #[test]
    fn anything_else_is_english() {
        assert!(!decide(&[None, None, None, Some("C")]));
        assert!(!decide(&[None, None, None, Some("de_DE.UTF-8")]));
        assert!(!decide(&[None, None, None, None]));
    }
}
