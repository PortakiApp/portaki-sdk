//! `check` as the gate of `release`, the `add` family, `upgrade`, the hidden former names, and
//! the CLI's language — on the real binary.

mod common;

use common::{document, module, text, Home};

fn bundles(dir: &std::path::Path, fr: &str, en: &str) {
    std::fs::create_dir_all(dir.join("i18n")).unwrap();
    std::fs::write(dir.join("i18n/fr-FR.json"), fr).unwrap();
    std::fs::write(dir.join("i18n/en-US.json"), en).unwrap();
}

/// A module whose conformance battery fails, formatted as rustfmt wants it.
fn failing_module() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"failing-check\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n",
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/lib.rs"), "//! Fixture.\n").unwrap();
    std::fs::create_dir_all(dir.path().join("tests")).unwrap();
    std::fs::write(
        dir.path().join("tests/conformance.rs"),
        "mod portaki_conformance {\n    #[test]\n    fn surfaces() {\n        panic!(\"home.card panicked\")\n    }\n}\n",
    )
    .unwrap();
    dir
}

/// `check` stops where `release` stops, with the same refusal.
#[test]
fn check_refuses_what_release_refuses() {
    let home = Home::new();
    let dir = failing_module();

    let output = home.run(dir.path(), &["--plain", "check"]);
    assert_eq!(output.status.code(), Some(1), "{}", text(&output.stdout));
    let stderr = text(&output.stderr);
    assert!(stderr.contains("tests before publish"), "{stderr}");
    assert!(stderr.contains("tests fail"), "{stderr}");
}

#[test]
fn check_only_runs_the_controls_named() {
    let home = Home::new();
    let dir = tempfile::tempdir().unwrap();
    module(dir.path(), "nuki");
    bundles(dir.path(), r#"{"a":"x","b":"y"}"#, r#"{"a":"x"}"#);

    let output = home.run(dir.path(), &["--json", "check", "--only", "i18n"]);
    assert_eq!(output.status.code(), Some(1));
    let check = document(&output);
    assert_eq!(check["controls"], serde_json::json!(["i18n"]));
    assert_eq!(check["modules"][0]["ok"], false);
    assert!(text(&output.stderr).contains("en-US.json: `b` missing"));

    let unknown = home.run(dir.path(), &["--plain", "check", "--only", "nope"]);
    assert_eq!(unknown.status.code(), Some(2));

    // The former name still runs the same control, and says the new one.
    let former = home.run(dir.path(), &["--plain", "i18n", "check"]);
    assert_eq!(former.status.code(), Some(1));
    assert!(text(&former.stderr).contains("portaki check --only i18n"));
}

#[test]
fn add_permission_connector_and_language() {
    let home = Home::new();
    let dir = tempfile::tempdir().unwrap();
    module(dir.path(), "nuki");
    bundles(dir.path(), r#"{"a":"x"}"#, r#"{"a":"y"}"#);

    let added = home.run(dir.path(), &["--plain", "add", "permission", "events"]);
    assert_eq!(added.status.code(), Some(0), "{}", text(&added.stderr));
    let cargo = std::fs::read_to_string(dir.path().join("Cargo.toml")).unwrap();
    assert!(cargo.contains("\"events\""), "{cargo}");
    let again = home.run(dir.path(), &["--plain", "add", "permission", "events"]);
    assert_eq!(again.status.code(), Some(3));
    let former = home.run(dir.path(), &["--plain", "permissions", "add", "email"]);
    assert_eq!(former.status.code(), Some(0));
    assert!(text(&former.stderr).contains("portaki add permission"));

    let connector = home.run(dir.path(), &["--plain", "add", "connector", "open-weather"]);
    assert_eq!(
        connector.status.code(),
        Some(0),
        "{}",
        text(&connector.stderr)
    );
    let lib = std::fs::read_to_string(dir.path().join("src/lib.rs")).unwrap();
    assert!(
        lib.contains("#[portaki_sdk::connector(builtin = \"open-weather\")]"),
        "{lib}"
    );
    let again = home.run(dir.path(), &["--plain", "add", "connector", "open-weather"]);
    assert_eq!(again.status.code(), Some(3));
    let unknown = home.run(dir.path(), &["--plain", "add", "connector", "my-api"]);
    assert_eq!(unknown.status.code(), Some(2));
    assert!(text(&unknown.stderr).contains("custom_connector"));

    let language = home.run(dir.path(), &["--plain", "add", "language", "de"]);
    assert_eq!(
        language.status.code(),
        Some(0),
        "{}",
        text(&language.stderr)
    );
    let de = std::fs::read_to_string(dir.path().join("i18n/de-DE.json")).unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&de).unwrap(),
        serde_json::json!({ "a": "" })
    );
    // What is left to write is what the gate refuses.
    let check = home.run(dir.path(), &["--plain", "check", "--only", "i18n"]);
    assert!(text(&check.stdout).contains("de-DE.json: `a` empty"));
}

/// The former names run, hidden from the root help.
#[test]
fn the_root_help_shows_the_new_names_only() {
    let home = Home::new();
    let dir = tempfile::tempdir().unwrap();

    let help = text(&home.run(dir.path(), &["--plain", "--help"]).stdout);
    for shown in ["check", "add", "upgrade", "run", "preview", "reports"] {
        assert!(help.contains(&format!("\n  {shown} ")), "{shown}: {help}");
    }
    for hidden in ["lint", "i18n", "permissions", "sdk", "publish"] {
        assert!(
            !help.contains(&format!("\n  {hidden} ")),
            "{hidden}: {help}"
        );
    }
    let upgrade = home.run(dir.path(), &["--plain", "upgrade", "--help"]);
    assert!(text(&upgrade.stdout).contains("--to"));
}

/// The system's language decides what a person reads; `--json` and exit codes do not move.
#[test]
fn a_french_system_reads_french_and_json_stays_the_same() {
    let home = Home::new();
    let dir = tempfile::tempdir().unwrap();
    module(dir.path(), "nuki");
    bundles(dir.path(), r#"{"a":"x","b":"y"}"#, r#"{"a":"x"}"#);

    let french = home.run_in("fr_FR.UTF-8", dir.path(), &["check", "--only", "i18n"]);
    assert_eq!(french.status.code(), Some(1));
    let said = text(&french.stdout) + &text(&french.stderr);
    assert!(
        said.contains("La porte que portaki release applique"),
        "{said}"
    );
    assert!(said.contains("texte(s) manquant(s)"), "{said}");

    let english = home.run_in("en_US.UTF-8", dir.path(), &["check", "--only", "i18n"]);
    assert!(text(&english.stderr).contains("text(s) missing"));

    let json_fr = home.run_in(
        "fr_FR.UTF-8",
        dir.path(),
        &["--json", "check", "--only", "i18n"],
    );
    let json_en = home.run_in(
        "en_US.UTF-8",
        dir.path(),
        &["--json", "check", "--only", "i18n"],
    );
    assert_eq!(json_fr.status.code(), json_en.status.code());
    assert_eq!(document(&json_fr), document(&json_en));

    // A usage error keeps its code in both languages.
    let usage = home.run_in(
        "fr_FR.UTF-8",
        dir.path(),
        &["--plain", "add", "language", "1"],
    );
    assert_eq!(usage.status.code(), Some(2));
    assert!(text(&usage.stderr).contains("n'est pas une langue"));
}
