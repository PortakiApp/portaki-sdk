//! `portaki test` — cargo test wrapper.

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};
use clap::Parser;

use crate::ui;

#[derive(Debug, Parser)]
/// Arguments for `portaki test`.
pub struct TestArgs {
    /// Extra arguments forwarded to `cargo test`.
    #[arg(last = true)]
    pub cargo_args: Vec<String>,
}

/// Runs `portaki test`.
pub fn run(args: TestArgs) -> Result<()> {
    ui::header(
        "portaki test",
        "Forward to cargo test — the module's own tests, on the host target.",
    );

    let mut cmd = Command::new("cargo");
    cmd.arg("test");
    for arg in &args.cargo_args {
        cmd.arg(arg);
    }

    // La sortie de `cargo test` est le sujet de la commande : elle passe en direct, sans
    // indicateur pour la masquer.
    let status = cmd.status().context("cargo test")?;
    ui::blank();
    if status.success() {
        ui::success("tests passed");
        ui::blank();
        Ok(())
    } else {
        anyhow::bail!("cargo test failed");
    }
}

/// The name every test of the battery carries: `portaki_conformance::manifest`, …
///
/// `portaki_test_utils::conformance!()` generates them in this module; libtest prints the path.
const CONFORMANCE_MODULE: &str = "portaki_conformance::";

/// Why a publication was refused before anything was built.
#[derive(Debug, PartialEq, Eq)]
pub enum Refusal {
    /// `cargo test` failed — tests, or the compilation of a test target.
    Failed,
    /// Everything passed, and the conformance battery was not among it.
    NoConformance,
}

impl Refusal {
    /// What the person publishing reads: the rule, and what to do about it.
    pub fn message(&self) -> String {
        match self {
            Refusal::Failed => "the module's tests fail — portaki publish does not push a module \
                 whose tests do not pass, and there is no flag to make it. Fix them, then publish \
                 again; portaki test runs the same thing"
                .to_string(),
            Refusal::NoConformance => format!(
                "the module's tests pass, but the conformance battery did not run — portaki \
                 publish requires it. Add tests/conformance.rs containing \
                 `portaki_test_utils::conformance!();` (portaki-test-utils {} or later)",
                env!("CARGO_PKG_VERSION")
            ),
        }
    }
}

/// `cargo test` as `portaki publish` runs it: the module's own crate, on the host target.
///
/// Every target cargo tests by default — unit tests, `tests/*.rs` (the conformance battery among
/// them), doctests. `CARGO_BUILD_TARGET` is dropped: publishing just compiled for wasm32, and a
/// leftover target would have cargo build tests it cannot run.
pub fn publish_test_command(module_root: &Path) -> Command {
    let mut cmd = Command::new("cargo");
    cmd.arg("test")
        .arg("--no-fail-fast")
        .current_dir(module_root)
        .env_remove("CARGO_BUILD_TARGET");
    cmd
}

/// Whether the conformance battery ran, read from libtest's output.
pub fn conformance_ran(stdout: &str) -> bool {
    stdout.lines().any(|line| {
        line.trim_start()
            .strip_prefix("test ")
            .is_some_and(|rest| rest.starts_with(CONFORMANCE_MODULE))
    })
}

/// The verdict on one `cargo test` run.
pub fn verdict(success: bool, stdout: &str) -> Result<(), Refusal> {
    if !success {
        Err(Refusal::Failed)
    } else if !conformance_ran(stdout) {
        Err(Refusal::NoConformance)
    } else {
        Ok(())
    }
}

/// Runs the module's tests and refuses the publication unless they pass, battery included.
///
/// Called by `portaki publish` before it builds or pushes anything — `--dry-run` and
/// `--skip-build` included: tests are not an artifact a previous job can hand over.
pub fn gate_publish(module_root: &Path) -> Result<()> {
    let step = ui::step("running the module's tests (conformance battery included)");
    let output = publish_test_command(module_root)
        .output()
        .context("run cargo test")?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    match verdict(output.status.success(), &stdout) {
        Ok(()) => {
            if ui::verbose() {
                ui::emit_captured(&output.stdout);
            }
            step.done("tests passed, conformance battery included");
            Ok(())
        }
        Err(refusal) => {
            step.abandon();
            // La sortie des tests d'abord, puis celle de cargo : elle finit par la cible qui a
            // échoué, c'est la dernière chose à lire avant le refus.
            ui::emit_captured(&output.stdout);
            if refusal == Refusal::Failed {
                ui::emit_captured(&output.stderr);
            }
            anyhow::bail!(refusal.message())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn the_battery_is_recognised_in_libtest_output() {
        let stdout = "\nrunning 5 tests\ntest portaki_conformance::manifest ... ok\n\
                      test portaki_conformance::i18n ... ok\n";
        assert!(conformance_ran(stdout));
    }

    /// A test merely mentioning the battery, or a module named alike, is not the battery.
    #[test]
    fn a_lookalike_is_not_the_battery() {
        let stdout = "test tests::portaki_conformance_is_documented ... ok\n\
                      test my_portaki_conformance::manifest ... ok\n\
                      portaki_conformance::manifest\n";
        assert!(!conformance_ran(stdout));
    }

    #[test]
    fn failing_tests_refuse_whatever_ran() {
        let stdout = "test portaki_conformance::manifest ... FAILED\n";
        assert_eq!(verdict(false, stdout), Err(Refusal::Failed));
    }

    #[test]
    fn passing_tests_without_the_battery_refuse() {
        assert_eq!(
            verdict(true, "test settings::round_trip ... ok\n"),
            Err(Refusal::NoConformance)
        );
    }

    #[test]
    fn passing_tests_with_the_battery_publish() {
        assert_eq!(
            verdict(true, "test portaki_conformance::surfaces ... ok\n"),
            Ok(())
        );
    }

    #[test]
    fn the_refusals_say_what_to_do() {
        let failed = Refusal::Failed.message();
        assert!(failed.contains("tests fail"), "{failed}");
        assert!(failed.contains("portaki test"), "{failed}");

        let missing = Refusal::NoConformance.message();
        assert!(missing.contains("tests/conformance.rs"), "{missing}");
        assert!(
            missing.contains("portaki_test_utils::conformance!();"),
            "{missing}"
        );
        assert!(missing.contains(env!("CARGO_PKG_VERSION")), "{missing}");
    }

    #[test]
    fn the_tests_run_in_the_module_on_the_host() {
        let root = Path::new("/modules/weather");
        let cmd = publish_test_command(root);

        assert_eq!(cmd.get_program(), "cargo");
        let args: Vec<_> = cmd.get_args().collect();
        assert_eq!(args, ["test", "--no-fail-fast"]);
        assert_eq!(cmd.get_current_dir(), Some(root));
        assert!(cmd
            .get_envs()
            .any(|(key, value)| key == "CARGO_BUILD_TARGET" && value.is_none()));
    }

    /// A crate with no dependency at all: `cargo test` on it is real, and fast.
    fn crate_with_tests(name: &str, tests: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n\
                 [workspace]\n"
            ),
        )
        .unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/lib.rs"), "").unwrap();
        fs::create_dir_all(dir.path().join("tests")).unwrap();
        fs::write(dir.path().join("tests/module.rs"), tests).unwrap();
        dir
    }

    #[test]
    fn a_failing_test_refuses_the_publication() {
        let module = crate_with_tests(
            "failing-module",
            "mod portaki_conformance { #[test] fn manifest() {} }\n\
             #[test] fn settings_round_trip() { assert_eq!(1, 2, \"settings lost\"); }\n",
        );

        let refusal = gate_publish(module.path()).unwrap_err().to_string();

        assert!(refusal.contains("tests fail"), "{refusal}");
    }

    #[test]
    fn a_module_without_the_battery_is_refused() {
        let module = crate_with_tests(
            "unconformed-module",
            "#[test] fn settings_round_trip() {}\n",
        );

        let refusal = gate_publish(module.path()).unwrap_err().to_string();

        assert!(
            refusal.contains("conformance battery did not run"),
            "{refusal}"
        );
    }

    #[test]
    fn a_module_whose_tests_and_battery_pass_goes_through() {
        let module = crate_with_tests(
            "conformed-module",
            "mod portaki_conformance { #[test] fn manifest() {} #[test] fn surfaces() {} }\n\
             #[test] fn settings_round_trip() {}\n",
        );

        gate_publish(module.path()).unwrap();
    }
}
