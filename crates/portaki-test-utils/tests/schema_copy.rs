//! The schema the battery bundles is the SDK's, byte for byte.

#[test]
fn the_bundled_schema_is_the_repository_one() {
    let repository = concat!(env!("CARGO_MANIFEST_DIR"), "/../../schema/module.v1.json");
    // Outside the SDK repository (a crates.io download) there is nothing to compare with.
    let Ok(expected) = std::fs::read_to_string(repository) else {
        return;
    };

    assert!(
        expected == portaki_test_utils::conformance::MODULE_SCHEMA_V1,
        "crates/portaki-test-utils/schema/module.v1.json drifted from schema/module.v1.json — \
         copy it over: cp schema/module.v1.json crates/portaki-test-utils/schema/"
    );
}
