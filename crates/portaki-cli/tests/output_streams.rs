//! What goes to stdout and what goes to stderr, checked on the real binary.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::process::{Command, Output};

fn portaki(cwd: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_portaki"))
        .args(["--plain"])
        .args(args)
        .current_dir(cwd)
        .env("PORTAKI_NO_UPDATE_CHECK", "1")
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

/// A local server that answers every request with `body`, as JSON.
fn serve(body: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut buffer = [0; 8192];
            let _ = stream.read(&mut buffer);
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
        }
    });
    format!("http://{address}")
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

/// The release action v1 and the `portaki-modules` workflow `grep` stdout for this line.
#[test]
fn already_in_the_registry_stays_on_stdout() {
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
    let registry = serve(r#"[{"digest":"sha256:aaa","version":"0.1.0"}]"#);

    let output = portaki(
        root,
        &[
            "publish",
            "--prebuilt",
            "--no-announce",
            "--registry",
            "ghcr.io/someone",
            "--url",
            &registry,
        ],
    );

    assert!(output.status.success(), "{}", text(&output.stderr));
    assert!(
        text(&output.stdout).contains("already in the registry"),
        "stdout: {}\nstderr: {}",
        text(&output.stdout),
        text(&output.stderr)
    );
}
