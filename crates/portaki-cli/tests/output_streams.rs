//! What goes to stdout and what goes to stderr, checked on the real binary.

use std::path::Path;
use std::process::{Command, Output};

mod common;
use common::Fake;

fn portaki(cwd: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_portaki"))
        .args(["--plain"])
        .args(args)
        .current_dir(cwd)
        .env("PORTAKI_NO_UPDATE_CHECK", "1")
        .env("PORTAKI_LANG", "en")
        .env_remove("PORTAKI_API_URL")
        .env_remove("PORTAKI_DEV_TOKEN")
        .env_remove("CI")
        .env_remove("GITHUB_ACTIONS")
        .env_remove("PORTAKI_PUBLISH_VERSION")
        .output()
        .unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

/// A warning goes to stderr: stdout stays what a script reads.
#[test]
fn a_warning_goes_to_stderr() {
    let home = tempfile::tempdir().unwrap();
    // Nothing listens on port 1: the revocation fails, and logout warns.
    std::fs::write(
        home.path().join("credentials.json"),
        r#"{"accessToken":"a","refreshToken":"r","origin":"http://127.0.0.1:1"}"#,
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_portaki"))
        .args(["--plain", "--api", "http://127.0.0.1:1", "logout"])
        .env(
            "PORTAKI_CREDENTIALS_FILE",
            home.path().join("credentials.json"),
        )
        .env("PORTAKI_NO_UPDATE_CHECK", "1")
        .env("PORTAKI_LANG", "en")
        .env_remove("PORTAKI_CREDENTIALS")
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", text(&output.stderr));
    assert!(text(&output.stderr).contains("could not reach the platform"));
    assert!(!text(&output.stdout).contains("could not reach the platform"));
}

/// `--url` was hidden and ignored: it is gone, and refused.
#[test]
fn logout_takes_no_url() {
    let dir = tempfile::tempdir().unwrap();
    let output = portaki(dir.path(), &["logout", "--url", "https://example.com"]);

    assert_eq!(output.status.code(), Some(2));
}

/// A module built for `ci release`: a publish manifest and a wasm, on SDK 8.7.0.
fn built_module() -> tempfile::TempDir {
    let module = tempfile::tempdir().unwrap();
    let root = module.path();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"streams\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n",
    )
    .unwrap();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/lib.rs"), "").unwrap();
    let artifact = root.join("target/portaki");
    std::fs::create_dir_all(&artifact).unwrap();
    std::fs::write(
        artifact.join("publish-manifest.json"),
        r#"{"id":"streams","version":"0.1.0","sdkVersion":"8.7.0"}"#,
    )
    .unwrap();
    let wasm = root.join("target/wasm32-unknown-unknown/release");
    std::fs::create_dir_all(&wasm).unwrap();
    std::fs::write(wasm.join("streams.wasm"), b"\0asm").unwrap();
    std::fs::write(
        root.join("Cargo.lock"),
        "[[package]]\nname = \"portaki-sdk\"\nversion = \"8.7.0\"\n",
    )
    .unwrap();
    module
}

fn sha256(bytes: &[u8]) -> String {
    use sha2::Digest as _;
    format!("sha256:{:x}", sha2::Sha256::digest(bytes))
}

/// The OCI manifest a push of `built_module` writes — with `wasm` as the module's bytes — and
/// its digest.
fn oci_manifest(wasm: &[u8]) -> (String, String) {
    let catalog: &[u8] = br#"{"id":"streams","version":"0.1.0","sdkVersion":"8.7.0"}"#;
    let manifest = serde_json::json!({
        "schemaVersion": 2,
        "mediaType": "application/vnd.oci.image.manifest.v1+json",
        "config": {
            "mediaType": "application/vnd.oci.empty.v1+json",
            "digest": sha256(b"{}"),
            "size": 2
        },
        "layers": [
            {
                "mediaType": "application/vnd.portaki.manifest+json",
                "digest": sha256(catalog),
                "size": catalog.len()
            },
            { "mediaType": "application/wasm", "digest": sha256(wasm), "size": wasm.len() }
        ]
    })
    .to_string();
    let digest = sha256(manifest.as_bytes());
    (manifest, digest)
}

/// Portaki's OCI repository holding `manifest` under every reference, and the platform in front of
/// it: `versions` as the catalogue lists them, the push refused with `push_refusal`.
fn registry_holding(manifest: String, versions: String, push_refusal: &str) -> (Fake, Fake) {
    let oci = Fake::start(vec![("/v2/modules/streams/manifests/", manifest)]);
    let host = oci.url.trim_start_matches("http://").to_string();
    let api = Fake::answering(vec![
        ("/registry/v1/modules/streams/versions", 200, versions),
        (
            "/registry/v1/modules/streams",
            200,
            serde_json::json!({ "artifactRef": format!("oci://{host}/modules/streams@sha256:0") })
                .to_string(),
        ),
        (
            "POST /registry/v1/publications/push-token",
            409,
            push_refusal.to_string(),
        ),
    ]);
    (oci, api)
}

fn ci_release(root: &Path, api: &str, outputs: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_portaki"))
        .args(["--plain", "--api", api, "ci", "release", "--no-sign"])
        .current_dir(root)
        .env("PORTAKI_NO_UPDATE_CHECK", "1")
        .env("PORTAKI_LANG", "en")
        .env("GITHUB_OUTPUT", outputs)
        .env("PORTAKI_DEV_TOKEN", "token")
        .env_remove("PORTAKI_PUBLISH_VERSION")
        .env_remove("ACTIONS_ID_TOKEN_REQUEST_URL")
        .output()
        .unwrap()
}

const VERSION_TAKEN: &str = r#"{"code":"version_already_published","message":"streams@0.1.0 existe déjà sur le canal stable"}"#;

/// `ci release` on a version already in the registry, with the same content: nothing is pushed,
/// the line stays on stdout, and the outcome reaches `GITHUB_OUTPUT` for the next step.
#[test]
fn already_in_the_registry_stays_on_stdout() {
    let module = built_module();
    let (manifest, digest) = oci_manifest(b"\0asm");
    let versions = serde_json::json!([{ "digest": digest, "version": "0.1.0" }]).to_string();
    let (_oci, api) = registry_holding(manifest, versions, VERSION_TAKEN);
    let outputs = module.path().join("github-output");

    let output = ci_release(module.path(), &api.url, &outputs);

    assert!(output.status.success(), "{}", text(&output.stderr));
    assert!(
        text(&output.stdout).contains("already in the registry"),
        "stdout: {}\nstderr: {}",
        text(&output.stdout),
        text(&output.stderr)
    );
    let written = std::fs::read_to_string(&outputs).unwrap();
    assert!(written.contains("outcome=already-published"), "{written}");
    assert!(written.contains(&format!("digest={digest}")), "{written}");
    // Settled from the catalogue: no right to push was even asked for.
    assert!(!api.seen().iter().any(|seen| seen.contains("push-token")));
}

/// The failure seen on a release replayed after a cancelled run: a draft is invisible to the
/// catalogue, the right to push is refused with a 409 — the tag is read and compared, and the
/// run is green.
#[test]
fn a_draft_already_published_with_the_same_content_is_green() {
    let module = built_module();
    let (manifest, digest) = oci_manifest(b"\0asm");
    let (oci, api) = registry_holding(manifest, "[]".to_string(), VERSION_TAKEN);
    let outputs = module.path().join("github-output");

    let output = ci_release(module.path(), &api.url, &outputs);

    assert!(output.status.success(), "{}", text(&output.stderr));
    let written = std::fs::read_to_string(&outputs).unwrap();
    assert!(written.contains("outcome=already-published"), "{written}");
    assert!(written.contains(&format!("digest={digest}")), "{written}");
    assert!(
        oci.seen()
            .iter()
            .any(|seen| seen.starts_with("/v2/modules/streams/manifests/0.1.0")),
        "{:?}",
        oci.seen()
    );
}

/// Other bytes under the same number are a change shipped without a version: refused.
#[test]
fn a_version_published_with_other_content_fails() {
    let module = built_module();
    let (manifest, digest) = oci_manifest(b"\0asm-from-another-build");
    for versions in [
        serde_json::json!([{ "digest": digest, "version": "0.1.0" }]).to_string(),
        "[]".to_string(),
    ] {
        let (_oci, api) = registry_holding(manifest.clone(), versions, VERSION_TAKEN);
        let outputs = module.path().join("github-output");

        let output = ci_release(module.path(), &api.url, &outputs);

        assert!(!output.status.success());
        assert!(
            text(&output.stderr).contains("already exists with other content"),
            "{}",
            text(&output.stderr)
        );
        let written = std::fs::read_to_string(&outputs).unwrap();
        assert!(written.contains("outcome=failed"), "{written}");
    }
}
