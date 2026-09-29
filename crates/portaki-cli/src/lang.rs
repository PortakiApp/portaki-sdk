//! The CLI's language: the machine's, French or English.
//!
//! `PORTAKI_LANG` first, then `LC_ALL`, `LC_MESSAGES` and `LANG` — the POSIX order. A value that
//! starts with `fr` gives French, with the words of the developer space; anything else gives
//! English. Only what a person reads changes: `--json` (always English), the platform's error
//! codes and the exit codes stay the same in both languages.
//!
//! Every sentence is written where it is used, both languages side by side
//! ([`tr!`](crate::tr)): a sentence added without its translation does not compile.

use std::sync::OnceLock;

/// The variables read, in the order in which they win.
const VARIABLES: [&str; 4] = ["PORTAKI_LANG", "LC_ALL", "LC_MESSAGES", "LANG"];

/// Does the CLI speak French?
///
/// Never under `--json`: the document and the messages it carries (`error`, `reason`) stay the
/// same whatever the machine, and so does what goes out on stderr alongside it.
pub fn french() -> bool {
    static FRENCH: OnceLock<bool> = OnceLock::new();
    !crate::ui::json()
        && *FRENCH.get_or_init(|| {
            // Unit tests do not follow the machine: the sentences they expect are English.
            let variables: &[&str] = if cfg!(test) {
                &VARIABLES[..1]
            } else {
                &VARIABLES
            };
            is_french(variables.iter().map(|key| std::env::var(key).ok()))
        })
}

/// The first non-empty value decides.
fn is_french(values: impl Iterator<Item = Option<String>>) -> bool {
    values
        .flatten()
        .find(|value| !value.trim().is_empty())
        .is_some_and(|value| value.trim().to_ascii_lowercase().starts_with("fr"))
}

/// The sentence in the CLI's language: `tr!("english {x}", "français {x}", …)`.
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

    /// A variable set to an empty value decides nothing: the next one speaks.
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
