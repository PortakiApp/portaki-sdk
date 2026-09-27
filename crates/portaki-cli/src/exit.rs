//! Les codes de sortie, tels que le README les documente.
//!
//! | Code | Sens |
//! |------|------|
//! | 0 | fait |
//! | 1 | échec |
//! | 2 | usage : argument refusé, module ambigu ou inconnu, profil inconnu |
//! | 3 | rien à faire, ou déjà fait |
//! | 130 | interrompu (ctrl-c) |

use std::sync::atomic::{AtomicBool, Ordering};

pub const FAILURE: i32 = 1;
pub const USAGE: i32 = 2;
pub const NOTHING_TO_DO: i32 = 3;

/// Une erreur d'usage : la commande n'a rien tenté, c'est la ligne de commande qui est à revoir.
#[derive(Debug)]
pub struct Usage(pub String);

impl std::fmt::Display for Usage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Usage {}

pub fn usage(message: impl Into<String>) -> anyhow::Error {
    Usage(message.into()).into()
}

static NOTHING_DONE: AtomicBool = AtomicBool::new(false);

/// La commande réussit sans avoir rien changé : elle sortira en 3.
pub fn nothing_to_do() {
    NOTHING_DONE.store(true, Ordering::Relaxed);
}

/// Le code d'une commande qui a abouti.
pub fn success_code() -> i32 {
    if NOTHING_DONE.load(Ordering::Relaxed) {
        NOTHING_TO_DO
    } else {
        0
    }
}

/// Le code d'une commande qui a échoué.
pub fn code(failure: &anyhow::Error) -> i32 {
    if failure.chain().any(|cause| cause.is::<Usage>()) {
        USAGE
    } else {
        FAILURE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_usage_error_keeps_its_code_under_context() {
        let failure = usage("pass --module").context("build");
        assert_eq!(code(&failure), USAGE);
        assert_eq!(code(&anyhow::anyhow!("boom")), FAILURE);
    }
}
