//! A manifest the registry would refuse fails the `manifest` check, field by field.

mod common;

use common::{assert_reports, broken, failing};

#[test]
fn a_manifest_off_schema_is_reported_where_it_breaks() {
    let findings = failing("manifest", broken().check_manifest());

    // `icon` is required.
    assert_reports(&findings, &["portaki.module.json at /", "icon"]);
    // `id` must be kebab-case.
    assert_reports(&findings, &["/id", "Fixture_Broken"]);
    // `version` must be semver.
    assert_reports(&findings, &["/version", "one"]);
    // Host surface types are a closed list.
    assert_reports(&findings, &["/hostSurfaces/0/type", "property-stats-strip"]);
    // A `pathSegment` ends up in API URLs: nothing that leaves its segment.
    assert_reports(&findings, &["/hostSurfaces/1/pathSegment", "auth/logout"]);
    // The root is closed: keys the platform ignores (`runtime`, `artifacts`, `config`) are refused.
    assert_reports(&findings, &["portaki.module.json at /", "runtime"]);
    // `config` is allowed, but its field types are the closed list the host form renders.
    assert_reports(&findings, &["/config/fields/0/type", "password"]);
    let rendered = findings.to_string();
    assert!(
        rendered.starts_with("portaki conformance — manifest:"),
        "{rendered}"
    );
}

/// No build yet: `portaki build` writes the manifest and `portaki lint` validates it.
#[test]
fn a_module_not_yet_built_has_no_manifest_to_validate() {
    let empty = portaki_test_utils::conformance::Module::at(env!("CARGO_MANIFEST_DIR"));
    empty.check_manifest().unwrap();
}

#[test]
fn the_passing_manifest_passes() {
    common::passing().check_manifest().unwrap();
}
