// Cargo only sets `OUT_DIR` for a crate that has a build script, and that is where the SDK
// macros write the emissions `portaki build` assembles into the manifest. Without this file
// there is no manifest, whatever the sources declare.
fn main() {
    println!("cargo:rerun-if-changed=src/");
    println!("cargo:rerun-if-changed=i18n/");
}
