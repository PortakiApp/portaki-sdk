//! `run`, `scenarios`, `preview`, `reports`, `release status|notes` — on the real binary, against
//! a fake platform that answers the routes the developer space calls.

mod common;

use common::{document, module, text, Fake, Home};

fn signed_in(fake: &Fake) -> (Home, tempfile::TempDir) {
    let home = Home::new();
    home.credentials(&[(&fake.url, "local")]);
    let dir = tempfile::tempdir().unwrap();
    module(dir.path(), "nuki");
    (home, dir)
}

fn sent(fake: &Fake, prefix: &str) -> serde_json::Value {
    let request = fake
        .requests()
        .into_iter()
        .find(|request| request.starts_with(prefix))
        .unwrap_or_else(|| panic!("no {prefix} in {:?}", fake.requests()));
    let body = request.splitn(3, ' ').nth(2).unwrap_or("");
    serde_json::from_str(body).unwrap_or(serde_json::Value::Null)
}

#[test]
fn run_dispatches_on_the_sandbox_build_and_fails_when_the_runtime_refuses() {
    let fake = Fake::start(vec![(
        "POST /dev/v1/modules/nuki/dispatch",
        r#"{"runId":"r","hasResult":true,"resultJson":"{\"ok\":true}","durationMs":12,"errorCode":null,"publishedEvents":[],"capturedEffects":[],"hostCalls":[{"op":"kv.get","durationMicros":40,"errorCode":""}]}"#.to_string(),
    )]);
    let (home, dir) = signed_in(&fake);

    let output = home.run(
        dir.path(),
        &[
            "--json",
            "--api",
            &fake.url,
            "run",
            "getConfig",
            "--params",
            r#"{"a":1}"#,
            "--as",
            "guest",
        ],
    );
    assert!(output.status.success(), "{}", text(&output.stderr));
    let run = document(&output);
    assert_eq!(run["operation"], "getConfig");
    assert_eq!(run["run"]["durationMs"], 12);
    let body = sent(&fake, "POST /dev/v1/modules/nuki/dispatch");
    assert_eq!(body["kind"], "query");
    assert_eq!(body["paramsJson"], r#"{"a":1}"#);
    assert_eq!(body["as"], "guest");

    let refused = Fake::start(vec![(
        "POST /dev/v1/modules/nuki/dispatch",
        r#"{"durationMs":3,"errorCode":"fuel_exhausted","hostCalls":[]}"#.to_string(),
    )]);
    home.credentials(&[(&refused.url, "local")]);
    let output = home.run(
        dir.path(),
        &["--plain", "--api", &refused.url, "run", "getConfig"],
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(
        text(&output.stderr).contains("fuel_exhausted"),
        "{}",
        text(&output.stderr)
    );

    let bad = home.run(
        dir.path(),
        &["--plain", "--api", &fake.url, "run", "x", "--params", "{"],
    );
    assert_eq!(bad.status.code(), Some(2));
}

#[test]
fn scenarios_show_replay_and_reset() {
    let grid = r#"[{"surface":"guest.home","case":"normal","status":"ok"},{"surface":"guest.home","case":"no_email","status":"fail","code":"render_failed","message":"panic"}]"#;
    let fake = Fake::start(vec![
        ("GET /dev/v1/modules/nuki/scenarios", grid.to_string()),
        ("POST /dev/v1/modules/nuki/scenarios/run", grid.to_string()),
        (
            "POST /dev/v1/sandbox/fixtures/reset",
            r#"{"generation":4}"#.to_string(),
        ),
    ]);
    let (home, dir) = signed_in(&fake);

    let shown = home.run(dir.path(), &["--json", "--api", &fake.url, "scenarios"]);
    assert_eq!(
        shown.status.code(),
        Some(1),
        "a failing case fails the command"
    );
    assert_eq!(document(&shown)["cells"][1]["case"], "no_email");

    let replayed = home.run(
        dir.path(),
        &["--plain", "--api", &fake.url, "scenarios", "run"],
    );
    assert!(
        text(&replayed.stderr).contains("render_failed"),
        "{}",
        text(&replayed.stderr)
    );
    assert!(fake
        .requests()
        .iter()
        .any(|r| r.starts_with("POST /dev/v1/modules/nuki/scenarios/run")));

    let reset = home.run(
        dir.path(),
        &["--json", "--api", &fake.url, "scenarios", "reset"],
    );
    assert!(reset.status.success(), "{}", text(&reset.stderr));
    assert_eq!(document(&reset)["generation"], 4);
}

#[test]
fn preview_renders_a_surface_and_says_the_build_never_reaches_production() {
    let fake = Fake::start(vec![
        (
            "GET /dev/v1/modules/nuki/surfaces",
            r#"[{"id":"guest.home","guest":true,"type":"guest_tab","types":["guest_tab"]}]"#.to_string(),
        ),
        (
            "POST /dev/v1/modules/nuki/surfaces/guest.home/render",
            r#"{"surfaceId":"guest.home","guest":true,"type":"guest_tab","types":["guest_tab"],"rendered":true,"tree":"{\"type\":\"text\"}","errorCode":null}"#.to_string(),
        ),
    ]);
    let (home, dir) = signed_in(&fake);

    let listed = home.run(dir.path(), &["--plain", "--api", &fake.url, "preview"]);
    assert!(
        text(&listed.stdout).contains("guest.home"),
        "{}",
        text(&listed.stdout)
    );

    let output = home.run(
        dir.path(),
        &[
            "--json",
            "--api",
            &fake.url,
            "preview",
            "guest.home",
            "--stay",
            "s-1",
        ],
    );
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert_eq!(document(&output)["preview"]["rendered"], true);
    assert!(text(&output.stderr).contains("never run in production"));
    assert_eq!(
        sent(
            &fake,
            "POST /dev/v1/modules/nuki/surfaces/guest.home/render"
        )["stayId"],
        "s-1"
    );
}

#[test]
fn reports_list_the_open_ones_and_resolve_with_an_internal_note() {
    let fake = Fake::start(vec![
        (
            "GET /dev/v1/modules/nuki/reports",
            r#"{"items":[{"id":"7f1","type":"error","status":"open","title":"render_failed","count":3}],"page":0,"size":50,"total":1,"counts":{"open":1}}"#.to_string(),
        ),
        (
            "PATCH /dev/v1/reports/7f1",
            r#"{"id":"7f1","status":"resolved","title":"render_failed"}"#.to_string(),
        ),
    ]);
    let (home, dir) = signed_in(&fake);

    let listed = home.run(
        dir.path(),
        &["--json", "--api", &fake.url, "reports", "--open"],
    );
    assert!(listed.status.success(), "{}", text(&listed.stderr));
    assert_eq!(
        document(&listed)["modules"][0]["reports"]["items"][0]["id"],
        "7f1"
    );
    assert!(fake.seen().iter().any(|line| line.contains("status=open")));

    let resolved = home.run(
        dir.path(),
        &[
            "--plain",
            "--api",
            &fake.url,
            "reports",
            "resolve",
            "7f1",
            "--note",
            "fixed in 1.2.1",
        ],
    );
    assert!(resolved.status.success(), "{}", text(&resolved.stderr));
    let body = sent(&fake, "PATCH /dev/v1/reports/7f1");
    assert_eq!(body["status"], "resolved");
    assert_eq!(body["internalNote"], "fixed in 1.2.1");

    let without = home.run(dir.path(), &["--plain", "reports", "resolve", "7f1"]);
    assert_eq!(without.status.code(), Some(2));
}

const VERSIONS: &str = r#"[{"digest":"sha256:d","version":"1.2.0","channel":"stable","status":"unofficial","yanked":false}]"#;

const DRAFT: &str = r#"{"digest":"sha256:d","moduleId":"nuki","version":"1.2.0","channel":"stable","releaseState":"draft","langs":["fr","en"],"added":[],"notes":{"changelog":[{"en":"Faster sync"}],"permissionReasons":{},"hostActionRequired":false,"hostAction":{}},"missing":[{"kind":"changelog","lang":"fr"}],"supplyChain":{"signature":"signed","signatureSource":"ci"}}"#;

#[test]
fn release_status_names_what_the_draft_misses_and_the_next_command() {
    let fake = Fake::start(vec![
        ("GET /dev/v1/modules/nuki/versions", VERSIONS.to_string()),
        (
            "GET /dev/v1/publications/sha256:d/release",
            DRAFT.to_string(),
        ),
    ]);
    let (home, dir) = signed_in(&fake);

    let status = document(&home.run(
        dir.path(),
        &["--json", "--api", &fake.url, "release", "status", "1.2.0"],
    ));
    assert_eq!(status["state"], "draft");
    assert_eq!(status["signature"], "signed");
    assert_eq!(status["missing"][0]["lang"], "fr");
    assert_eq!(
        status["next"]["command"],
        "portaki release notes 1.2.0 --complete"
    );

    let absent = home.run(
        dir.path(),
        &["--plain", "--api", &fake.url, "release", "status", "9.9.9"],
    );
    assert_eq!(absent.status.code(), Some(1));
}

#[test]
fn release_notes_complete_the_draft_and_report_what_the_registry_still_refuses() {
    let available = DRAFT
        .replace(r#""releaseState":"draft""#, r#""releaseState":"available""#)
        .replace(r#"[{"kind":"changelog","lang":"fr"}]"#, "[]");
    let fake = Fake::start(vec![
        ("GET /dev/v1/modules/nuki/versions", VERSIONS.to_string()),
        (
            "GET /dev/v1/publications/sha256:d/release",
            DRAFT.to_string(),
        ),
        ("PUT /dev/v1/publications/sha256:d/release", available),
    ]);
    let (home, dir) = signed_in(&fake);

    let output = home.run(
        dir.path(),
        &[
            "--json",
            "--api",
            &fake.url,
            "release",
            "notes",
            "1.2.0",
            "--notes",
            "fr:Synchro plus rapide",
        ],
    );
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert_eq!(document(&output)["state"], "available");
    let body = sent(&fake, "PUT /dev/v1/publications/sha256:d/release");
    assert_eq!(
        body["changelog"],
        serde_json::json!([{ "en": "Faster sync", "fr": "Synchro plus rapide" }])
    );

    // A commit line is refused before anything is sent.
    let commit = home.run(
        dir.path(),
        &[
            "--plain",
            "--api",
            &fake.url,
            "release",
            "notes",
            "1.2.0",
            "--notes",
            "fr:fix: sync",
        ],
    );
    assert_eq!(commit.status.code(), Some(1));

    let refusing = Fake::answering(vec![
        ("GET /dev/v1/modules/nuki/versions", 200, VERSIONS.to_string()),
        ("GET /dev/v1/publications/sha256:d/release", 200, DRAFT.to_string()),
        (
            "PUT /dev/v1/publications/sha256:d/release",
            409,
            r#"{"code":"changelog_incomplete","message":"il manque fr","missing":[{"kind":"changelog","lang":"fr"}]}"#.to_string(),
        ),
    ]);
    home.credentials(&[(&refusing.url, "local")]);
    let output = home.run(
        dir.path(),
        &[
            "--plain",
            "--api",
            &refusing.url,
            "release",
            "notes",
            "1.2.0",
            "--complete",
        ],
    );
    assert_eq!(output.status.code(), Some(1));
    let stderr = text(&output.stderr);
    assert!(stderr.contains("changelog (fr)"), "{stderr}");
    assert!(stderr.contains("draft"), "{stderr}");
}

/// The old name still works, hidden from the help, and `release` alone still publishes.
#[test]
fn the_former_names_are_hidden() {
    let home = Home::new();
    let dir = tempfile::tempdir().unwrap();

    let help = text(&home.run(dir.path(), &["--plain", "dev", "--help"]).stdout);
    assert!(!help.contains("--dispatch"), "{help}");

    let release = text(
        &home
            .run(dir.path(), &["--plain", "release", "--help"])
            .stdout,
    );
    assert!(release.contains("status"), "{release}");
    assert!(release.contains("--dry-run"), "{release}");
}
