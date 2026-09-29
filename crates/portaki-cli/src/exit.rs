//! The exit codes, as the README documents them.
//!
//! | Code | Meaning |
//! |------|---------|
//! | 0 | done |
//! | 1 | failure |
//! | 2 | usage: argument refused, ambiguous or unknown module, unknown profile |
//! | 3 | nothing to do, or already done |
//! | 130 | interrupted (ctrl-c) |

use std::sync::atomic::{AtomicBool, Ordering};

pub const FAILURE: i32 = 1;
pub const USAGE: i32 = 2;
pub const NOTHING_TO_DO: i32 = 3;

/// A usage error: the command attempted nothing, it is the command line that needs revisiting.
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

/// The command succeeds without having changed anything: it will exit with 3.
pub fn nothing_to_do() {
    NOTHING_DONE.store(true, Ordering::Relaxed);
}

/// The code of a command that went through.
pub fn success_code() -> i32 {
    if NOTHING_DONE.load(Ordering::Relaxed) {
        NOTHING_TO_DO
    } else {
        0
    }
}

/// The code of a command that failed.
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
