//! Shared by the conformance fixture tests.

#![allow(dead_code)]

use portaki_test_utils::conformance::{Findings, Module};

pub fn passing() -> Module {
    Module::at(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/passing"
    ))
}

pub fn broken() -> Module {
    Module::at(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/broken"
    ))
}

/// The findings of a check that must fail, with a message worth reading when it does not.
pub fn failing(check: &str, result: Result<(), Findings>) -> Findings {
    match result {
        Ok(()) => panic!("the {check} check passed where it should have failed"),
        Err(findings) => {
            assert_eq!(findings.checks(), vec![check], "{findings}");
            findings
        }
    }
}

/// One problem mentions every fragment — and the report says which, when none does.
pub fn assert_reports(findings: &Findings, fragments: &[&str]) {
    assert!(
        findings
            .problems()
            .iter()
            .any(|problem| fragments.iter().all(|fragment| problem.contains(fragment))),
        "no problem mentions all of {fragments:?}:\n{findings}"
    );
}
