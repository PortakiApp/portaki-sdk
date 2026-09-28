//! The fake platform and the isolated home every command test runs against.
#![allow(dead_code)]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};

/// A fake platform: answers by path prefix — `"PUT /x"` to match one method only — and remembers
/// what it was asked, with which token and which body.
pub struct Fake {
    pub url: String,
    seen: Arc<Mutex<Vec<String>>>,
    bodies: Arc<Mutex<Vec<String>>>,
}

impl Fake {
    /// Every route answers 200.
    pub fn start(routes: Vec<(&'static str, String)>) -> Self {
        Self::answering(
            routes
                .into_iter()
                .map(|(prefix, body)| (prefix, 200, body))
                .collect(),
        )
    }

    /// Each route with its own status.
    pub fn answering(routes: Vec<(&'static str, u16, String)>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let seen = Arc::new(Mutex::new(Vec::new()));
        let bodies = Arc::new(Mutex::new(Vec::new()));
        let (log, sent) = (Arc::clone(&seen), Arc::clone(&bodies));
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut request = String::new();
                reader.read_line(&mut request).unwrap_or_default();
                let mut words = request.split_whitespace();
                let method = words.next().unwrap_or("").to_string();
                let path = words.next().unwrap_or("").to_string();
                let mut auth = String::new();
                let mut length = 0;
                loop {
                    let mut header = String::new();
                    if reader.read_line(&mut header).unwrap_or(0) == 0 || header.trim().is_empty() {
                        break;
                    }
                    let lower = header.to_ascii_lowercase();
                    if let Some(value) = lower.strip_prefix("authorization:") {
                        auth = header[header.len() - value.len()..].trim().to_string();
                    }
                    if let Some(value) = lower.strip_prefix("content-length:") {
                        length = value.trim().parse().unwrap_or(0);
                    }
                }
                let mut body = vec![0; length];
                let _ = reader.read_exact(&mut body);
                log.lock().unwrap().push(format!("{path} {auth}"));
                sent.lock().unwrap().push(format!(
                    "{method} {path} {}",
                    String::from_utf8_lossy(&body)
                ));
                let found = routes
                    .iter()
                    .find(|(prefix, _, _)| match prefix.split_once(' ') {
                        Some((only, rest)) => only == method && path.starts_with(rest),
                        None => path.starts_with(prefix),
                    });
                let (status, answer) = found
                    .map(|(_, status, answer)| (*status, answer.as_str()))
                    .unwrap_or((404, "{}"));
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{answer}",
                    answer.len()
                );
            }
        });
        Fake { url, seen, bodies }
    }

    /// `<path> <authorization>`, one per request.
    pub fn seen(&self) -> Vec<String> {
        self.seen.lock().unwrap().clone()
    }

    /// `<METHOD> <path> <body>`, one per request.
    pub fn requests(&self) -> Vec<String> {
        self.bodies.lock().unwrap().clone()
    }
}

/// A home of its own: credentials, config and cache never touch the machine's.
pub struct Home(pub tempfile::TempDir);

impl Home {
    pub fn new() -> Self {
        Home(tempfile::tempdir().unwrap())
    }

    pub fn credentials(&self, sessions: &[(&str, &str)]) {
        let sessions: serde_json::Map<String, serde_json::Value> = sessions
            .iter()
            .map(|(origin, token)| {
                (
                    origin.to_string(),
                    serde_json::json!({ "accessToken": token, "refreshToken": format!("r-{token}") }),
                )
            })
            .collect();
        std::fs::create_dir_all(self.0.path().join("portaki")).unwrap();
        std::fs::write(
            self.0.path().join("portaki/credentials.json"),
            serde_json::json!({ "sessions": sessions }).to_string(),
        )
        .unwrap();
    }

    pub fn stored(&self) -> serde_json::Value {
        serde_json::from_str(
            &std::fs::read_to_string(self.0.path().join("portaki/credentials.json")).unwrap(),
        )
        .unwrap()
    }

    pub fn run(&self, cwd: &Path, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_portaki"))
            .args(args)
            .current_dir(cwd)
            .env("XDG_CONFIG_HOME", self.0.path())
            .env("XDG_CACHE_HOME", self.0.path())
            .env("PORTAKI_NO_UPDATE_CHECK", "1")
            .env("PORTAKI_COSIGN", "/nonexistent/cosign")
            .env("PORTAKI_GH", "/nonexistent/gh")
            .env_remove("PORTAKI_CREDENTIALS_FILE")
            .env_remove("PORTAKI_CREDENTIALS")
            .env_remove("PORTAKI_API_URL")
            .env_remove("PORTAKI_DEV_URL")
            .env_remove("PORTAKI_DEV_TOKEN")
            .env_remove("CI")
            .env_remove("GITHUB_ACTIONS")
            .stdin(std::process::Stdio::null())
            .output()
            .unwrap()
    }
}

pub fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

/// stdout holds exactly one JSON document — nothing before, nothing after.
pub fn document(output: &Output) -> serde_json::Value {
    let stdout = text(&output.stdout);
    assert_eq!(
        stdout.trim().lines().count(),
        1,
        "stdout: {stdout}\nstderr: {}",
        text(&output.stderr)
    );
    serde_json::from_str(stdout.trim()).unwrap_or_else(|failure| panic!("{failure}: {stdout}"))
}

pub fn module(dir: &Path, id: &str) {
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(
        dir.join("Cargo.toml"),
        format!(
            "[package]\nname = \"{id}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n\
             [dependencies]\nportaki-sdk = {{ version = \"8\", features = [\"kv\"] }}\n"
        ),
    )
    .unwrap();
    std::fs::write(dir.join("src/lib.rs"), "").unwrap();
}
