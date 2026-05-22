//! Compile-time UI tests for proc-macros (trybuild).

#[test]
fn capability_macro_ui() {
    let test_cases = trybuild::TestCases::new();
    test_cases.pass("tests/ui/capability/*.rs");
    test_cases.compile_fail("tests/ui/capability-fail/*.rs");
}
