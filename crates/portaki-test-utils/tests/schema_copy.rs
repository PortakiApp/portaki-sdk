//! The schemas the battery bundles are the SDK's, byte for byte.

fn assert_same(file: &str, bundled: &str) {
    let repository = format!("{}/../../schema/{file}", env!("CARGO_MANIFEST_DIR"));
    // Outside the SDK repository (a crates.io download) there is nothing to compare with.
    let Ok(expected) = std::fs::read_to_string(repository) else {
        return;
    };

    assert!(
        expected == bundled,
        "crates/portaki-test-utils/schema/{file} drifted from schema/{file} — \
         copy it over: cp schema/{file} crates/portaki-test-utils/schema/"
    );
}

#[test]
fn the_bundled_schema_is_the_repository_one() {
    assert_same(
        "module.v1.json",
        portaki_test_utils::conformance::MODULE_SCHEMA_V1,
    );
}

#[test]
fn the_bundled_listing_schema_is_the_repository_one() {
    assert_same(
        "listing.v1.json",
        portaki_test_utils::conformance::LISTING_SCHEMA_V1,
    );
}
