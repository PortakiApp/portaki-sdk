//! A module refuses with a code it declared — and the battery wants that code's label.

mod common;

use common::{assert_reports, failing, passing};
use portaki_sdk::prelude::*;
use portaki_test_utils::{assert_declared_error, assert_refused, MockContext};

portaki_sdk::portaki_module!(id = "refusals", error = "photo_required");

fn report(_ctx: Context, photo: Option<&str>) -> Result<()> {
    photo
        .map(|_| ())
        .ok_or(portaki_sdk::refuse!("photo_required"))
}

#[test]
fn a_declared_refusal_reaches_the_test() {
    let result = MockContext::guest().run(|ctx| report(ctx, None));
    assert_refused(&result, "photo_required");
    assert_eq!(
        result.unwrap_err().to_string(),
        "module_refused:photo_required"
    );
    assert_declared_error("photo_required");
}

#[test]
#[should_panic(expected = "expected Err(Refused(\"photo_required\"))")]
fn a_success_is_not_a_refusal() {
    let result = MockContext::guest().run(|ctx| report(ctx, Some("photo.jpg")));
    assert_refused(&result, "photo_required");
}

/// Built by hand, a refusal skips the compile-time check: the helper catches it.
#[test]
#[should_panic(expected = "refusal code `host_only` is not declared")]
fn a_hand_built_undeclared_refusal_fails_the_helper() {
    let result: Result<()> = Err(PortakiError::Refused("host_only"));
    assert_refused(&result, "host_only");
}

#[test]
fn a_declared_code_needs_its_label_in_every_bundle() {
    let findings = failing("i18n", passing().check_i18n());

    assert_reports(
        &findings,
        &[
            "`errors.photo_required`",
            "fr and en",
            "error = \"photo_required\"",
        ],
    );
}
