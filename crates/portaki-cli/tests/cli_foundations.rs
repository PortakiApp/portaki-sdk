//! Module selection, profiles and per-origin sessions, `--json`, `status`, `doctor` and exit
//! codes — on the real binary, against a local fake platform.

mod common;

use std::process::Command;

use common::{document, module, text, Fake, Home};

fn monorepo() -> tempfile::TempDir {
    let repo = tempfile::tempdir().unwrap();
    module(&repo.path().join("modules/nuki"), "nuki");
    module(&repo.path().join("modules/wifi"), "wifi");
    repo
}

#[test]
fn an_ambiguous_monorepo_is_a_usage_error_that_names_the_modules() {
    let home = Home::new();
    let repo = monorepo();

    let output = home.run(repo.path(), &["--plain", "lint"]);
    assert_eq!(output.status.code(), Some(2), "{}", text(&output.stderr));
    let stderr = text(&output.stderr);
    assert!(stderr.contains("--module <id> or --all"), "{stderr}");
    assert!(stderr.contains("nuki, wifi"), "{stderr}");

    let unknown = home.run(
        repo.path(),
        &["--plain", "ci", "info", "--module", "absent"],
    );
    assert_eq!(unknown.status.code(), Some(2));

    let all = home.run(repo.path(), &["--plain", "dev", "--all"]);
    assert_eq!(all.status.code(), Some(2));
    assert!(text(&all.stderr).contains("one module at a time"));
}

#[test]
fn the_same_selection_works_on_ci_with_json() {
    let home = Home::new();
    let repo = monorepo();

    let all = document(&home.run(repo.path(), &["--json", "ci", "info", "--all"]));
    let ids: Vec<&str> = all["modules"]
        .as_array()
        .unwrap()
        .iter()
        .map(|module| module["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, vec!["nuki", "wifi"]);

    let one = document(&home.run(repo.path(), &["--json", "ci", "info", "--module", "wifi"]));
    assert_eq!(one["modules"][0]["version"], "0.1.0");

    // `--root` stays accepted, hidden.
    let root = repo.path().join("modules/nuki");
    let legacy = document(&home.run(
        repo.path(),
        &["--json", "ci", "info", "--root", root.to_str().unwrap()],
    ));
    assert_eq!(legacy["modules"][0]["id"], "nuki");

    let modules = document(&home.run(
        repo.path(),
        &["--json", "ci", "modules", "--module", "nuki"],
    ));
    assert_eq!(modules["modules"], serde_json::json!(["nuki"]));
    assert_eq!(modules["any"], true);
}

#[test]
fn an_unknown_profile_is_a_usage_error() {
    let home = Home::new();
    let dir = tempfile::tempdir().unwrap();

    let output = home.run(dir.path(), &["--plain", "--env", "stagging", "status"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(text(&output.stderr).contains("prod, staging, local"));
}

/// A profile from the config file, and a command towards an origin without a session: it
/// names the login that opens one there.
#[test]
fn a_command_without_a_session_there_names_the_login_for_that_environment() {
    let home = Home::new();
    home.credentials(&[("https://api.portaki.app", "prod")]);
    let dir = tempfile::tempdir().unwrap();
    module(dir.path(), "nuki");

    let status = document(&home.run(dir.path(), &["--json", "--env", "staging", "status"]));
    assert_eq!(status["signedIn"], false);
    assert_eq!(status["api"], "https://api-staging.portaki.app");
    assert_eq!(status["next"]["command"], "portaki login --env staging");

    std::fs::write(
        home.0.path().join("portaki/config.toml"),
        "[env.preprod]\napi = \"https://pre.example\"\n",
    )
    .unwrap();
    let logs = home.run(dir.path(), &["--plain", "--env", "preprod", "logs"]);
    assert_eq!(logs.status.code(), Some(1));
    assert!(
        text(&logs.stderr).contains("portaki login --env preprod"),
        "{}",
        text(&logs.stderr)
    );
}

/// Each origin has its own session: the one sent is the one of the origin addressed.
#[test]
fn the_session_sent_is_the_one_of_the_origin_addressed() {
    let fake = Fake::start(vec![("/dev/v1/onboarding", "{}".to_string())]);
    let home = Home::new();
    home.credentials(&[("https://api.portaki.app", "prod"), (&fake.url, "local")]);
    let dir = tempfile::tempdir().unwrap();
    module(dir.path(), "nuki");

    let output = home.run(dir.path(), &["--json", "--api", &fake.url, "status"]);
    assert!(output.status.success(), "{}", text(&output.stderr));

    let seen = fake.seen();
    assert!(!seen.is_empty());
    assert!(
        seen.iter().all(|line| line.ends_with("Bearer local")),
        "{seen:?}"
    );
}

/// `login` against one origin leaves the others' sessions alone.
#[test]
fn signing_in_to_one_origin_keeps_the_others() {
    let fake = Fake::start(vec![
        (
            "/api/v1/auth/device/code",
            r#"{"success":true,"data":{"deviceCode":"d","userCode":"U","verificationUri":"http://127.0.0.1/x","expiresIn":30,"interval":1}}"#.to_string(),
        ),
        (
            "/api/v1/auth/device/token",
            r#"{"success":true,"data":{"accessToken":"new","refreshToken":"r-new"}}"#.to_string(),
        ),
    ]);
    let home = Home::new();
    home.credentials(&[("https://api.portaki.app", "prod")]);
    let dir = tempfile::tempdir().unwrap();

    let output = home.run(
        dir.path(),
        &["--plain", "--api", &fake.url, "login", "--no-browser"],
    );
    assert!(output.status.success(), "{}", text(&output.stderr));

    let stored = home.stored();
    assert_eq!(
        stored["sessions"]["https://api.portaki.app"]["accessToken"],
        "prod"
    );
    assert_eq!(stored["sessions"][&fake.url]["accessToken"], "new");

    // And logging out of it leaves production signed in.
    let _ = home.run(dir.path(), &["--plain", "--api", &fake.url, "logout"]);
    let stored = home.stored();
    assert!(stored["sessions"].get(&fake.url).is_none());
    assert_eq!(
        stored["sessions"]["https://api.portaki.app"]["accessToken"],
        "prod"
    );
}

#[test]
fn status_reads_the_journey_and_names_the_next_command() {
    let fake = Fake::start(vec![
        (
            "/dev/v1/onboarding",
            r#"{"cliConnected":true,"deployed":true,"rendered":true,"conformant":true,"published":true}"#.to_string(),
        ),
        (
            "/dev/v1/nav-counts",
            r#"{"modulesWithErrors":1,"reviewsToFix":0,"perModule":{"nuki":{"err24":4,"openReports":2,"failedChecks":0,"linked":true}}}"#.to_string(),
        ),
        (
            "/dev/v1/modules/nuki/link",
            r#"{"moduleId":"nuki","repository":"acme/nuki"}"#.to_string(),
        ),
        (
            "/dev/v1/modules/nuki/status",
            r#"{"watchConnected":false,"lastDeploy":{"version":"1.2.0","at":"2026-09-27T10:00:00Z"},"lastRun":{"status":"ok","at":"2026-09-27T10:01:00Z"}}"#.to_string(),
        ),
        (
            "/registry/v1/publications/mine",
            r#"{"items":[{"digest":"sha256:b","moduleId":"nuki","version":"1.2.0","channel":"stable","status":"approved","releaseState":"available","publishedAt":"2026-09-26T09:00:00Z"}]}"#.to_string(),
        ),
        (
            "/registry/v1/publications/sha256:b/release",
            r#"{"releaseState":"available","missing":[],"supplyChain":{"signature":"signed","signatureSource":"ci"}}"#.to_string(),
        ),
    ]);
    let home = Home::new();
    home.credentials(&[(&fake.url, "local")]);
    let dir = tempfile::tempdir().unwrap();
    module(dir.path(), "nuki");

    let output = home.run(dir.path(), &["--json", "--api", &fake.url, "status"]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    let status = document(&output);
    assert_eq!(status["schemaVersion"], 1);
    assert_eq!(status["journey"]["conformant"], true);
    let nuki = &status["modules"][0];
    assert_eq!(nuki["repository"], "acme/nuki");
    assert_eq!(nuki["err24"], 4);
    assert_eq!(nuki["openReports"], 2);
    assert_eq!(nuki["latest"]["signature"], "signed");
    assert_eq!(nuki["latest"]["signatureSource"], "ci");
    assert_eq!(nuki["latest"]["review"], "approved");
    assert_eq!(status["next"]["command"], "portaki logs");

    // The same, for a person: the command to run, last.
    let human = home.run(dir.path(), &["--plain", "--api", &fake.url, "status"]);
    let stdout = text(&human.stdout);
    assert!(stdout.contains("acme/nuki"), "{stdout}");
    assert!(
        stdout.contains("1.2.0 · stable · available · signed (ci) · review approved"),
        "{stdout}"
    );
}

#[test]
fn doctor_says_what_fails_and_how_to_fix_it() {
    let fake = Fake::start(vec![
        ("/dev/v1/onboarding", "{}".to_string()),
        (
            "/dev/v1/modules/nuki/link",
            r#"{"repository":"acme/nuki","requiredEnvironment":"release"}"#.to_string(),
        ),
    ]);
    let home = Home::new();
    let dir = tempfile::tempdir().unwrap();
    module(dir.path(), "nuki");
    std::fs::create_dir_all(dir.path().join(".cargo")).unwrap();
    std::fs::write(dir.path().join(".cargo/config.toml"), "").unwrap();
    std::fs::create_dir_all(dir.path().join(".git")).unwrap();

    // Signed out: the session fails, with the login that fixes it, and the exit code is 1.
    let output = home.run(
        dir.path(),
        &["--json", "--api", &fake.url, "doctor", "--offline"],
    );
    assert_eq!(output.status.code(), Some(1));
    let doctor = document(&output);
    assert_eq!(doctor["ok"], false);
    let check = |id: &str| {
        doctor["checks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|check| check["id"] == id)
            .cloned()
            .unwrap_or_else(|| panic!("no {id} in {doctor}"))
    };
    assert_eq!(check("session")["status"], "fail");
    assert_eq!(
        check("session")["fix"],
        format!("portaki login --api {}", fake.url)
    );
    assert_eq!(check("cosign")["status"], "warn");
    assert_eq!(check("overrides")["status"], "warn");
    assert!(check("overrides")["summary"]
        .as_str()
        .unwrap()
        .contains(".cargo/config.toml"));
    assert_eq!(check("overrides")["module"], "nuki");

    // Signed in: the session opens, the module is linked.
    home.credentials(&[(&fake.url, "local")]);
    let doctor = document(&home.run(
        dir.path(),
        &["--json", "--api", &fake.url, "doctor", "--offline"],
    ));
    let session = doctor["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|check| check["id"] == "session")
        .unwrap()
        .clone();
    assert_eq!(session["status"], "ok");
    let link = doctor["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|check| check["id"] == "link")
        .unwrap()
        .clone();
    assert_eq!(link["status"], "ok");
}

/// 3: nothing to do — the permission was already declared.
#[test]
fn nothing_to_do_exits_with_three() {
    let home = Home::new();
    let dir = tempfile::tempdir().unwrap();
    module(dir.path(), "nuki");

    let output = home.run(dir.path(), &["--plain", "permissions", "add", "kv"]);
    assert_eq!(output.status.code(), Some(3), "{}", text(&output.stderr));

    let added = home.run(dir.path(), &["--plain", "permissions", "add", "email"]);
    assert_eq!(added.status.code(), Some(0), "{}", text(&added.stderr));
}

/// `PORTAKI_DEV_URL` still works, as a deprecated alias, and says so on stderr.
#[test]
fn the_old_dev_variable_is_a_deprecated_alias() {
    let home = Home::new();
    let dir = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_portaki"))
        .args(["--json", "status"])
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", home.0.path())
        .env("PORTAKI_NO_UPDATE_CHECK", "1")
        .env("PORTAKI_LANG", "en")
        .env("PORTAKI_DEV_URL", "https://dev.example")
        .env_remove("PORTAKI_API_URL")
        .env_remove("PORTAKI_DEV_TOKEN")
        .env_remove("PORTAKI_CREDENTIALS_FILE")
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", text(&output.stderr));
    assert_eq!(document(&output)["api"], "https://dev.example");
    assert!(text(&output.stderr).contains("PORTAKI_DEV_URL is deprecated"));
}
