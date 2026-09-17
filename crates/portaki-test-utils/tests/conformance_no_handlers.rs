//! A test binary that links no handler must not pass on an empty list.

mod common;

use common::{assert_reports, failing, passing};

#[test]
fn surfaces_fail_when_no_handler_is_visible() {
    let findings = failing("surfaces", passing().check_surfaces());

    assert_reports(
        &findings,
        &["no #[surface]", "portaki-sdk-macros", "conformance!()"],
    );
}

#[test]
fn operations_fail_when_no_handler_is_visible() {
    let findings = failing("operations", passing().check_operations());

    assert_reports(&findings, &["no #[surface]"]);
}
