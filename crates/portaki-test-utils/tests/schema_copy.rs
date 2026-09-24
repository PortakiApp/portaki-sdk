//! The schemas the battery bundles are the SDK's, byte for byte.

fn assert_same(path: &str, bundled: &str) {
    let repository = format!("{}/../../{path}", env!("CARGO_MANIFEST_DIR"));
    // Outside the SDK repository (a crates.io download) there is nothing to compare with.
    let Ok(expected) = std::fs::read_to_string(repository) else {
        return;
    };

    assert!(
        expected == bundled,
        "crates/portaki-test-utils/schema/ drifted from {path} — \
         copy it over: cp {path} crates/portaki-test-utils/schema/"
    );
}

#[test]
fn the_bundled_schema_is_the_repository_one() {
    assert_same(
        "schema/module.v1.json",
        portaki_test_utils::conformance::MODULE_SCHEMA_V1,
    );
}

#[test]
fn the_bundled_listing_schema_is_the_repository_one() {
    assert_same(
        "schema/listing.v1.json",
        portaki_test_utils::conformance::LISTING_SCHEMA_V1,
    );
}

#[test]
fn the_bundled_contract_schemas_are_the_repository_ones() {
    use portaki_test_utils::conformance::{
        PUBLISH_READINESS_SCHEMA_V1, STATS_SUMMARY_SCHEMA_V1, TIMELINE_TASKS_SCHEMA_V1,
    };

    assert_same(
        "contracts/publish-readiness.v1.json",
        PUBLISH_READINESS_SCHEMA_V1,
    );
    assert_same("contracts/stats-summary.v1.json", STATS_SUMMARY_SCHEMA_V1);
    assert_same("contracts/timeline-tasks.v1.json", TIMELINE_TASKS_SCHEMA_V1);
}
