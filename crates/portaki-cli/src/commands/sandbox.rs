//! `portaki run`, `portaki scenarios`, `portaki preview` — the developer space's Sandbox page,
//! in the terminal.
//!
//! All three act on the last build `portaki dev` pushed, through the routes the developer space
//! already calls: `POST /dev/v1/modules/{id}/dispatch`,
//! `GET|POST /dev/v1/modules/{id}/scenarios[/run]`, `POST /dev/v1/sandbox/fixtures/reset`,
//! `GET /dev/v1/modules/{id}/surfaces` and `POST …/surfaces/{surface}/render`. None of them
//! compiles and none of them deploys: `portaki dev` is what does that.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use reqwest::Method;
use serde_json::{json, Value};

use crate::api::Platform;
use crate::commands::dev::{self, DispatchResponse, ScenarioCell};
use crate::ui;
use crate::workspace::ModuleArgs;

/// What a sandbox build amounts to outside the sandbox: nothing. Said on every render, because a
/// preview that worked looks exactly like a version that is ready.
pub fn sandbox_only() -> String {
    crate::tr!(
        "sandbox build, unsigned — it will never run in production; portaki release publishes a signed version",
        "build de sandbox, non signé — il ne s'exécutera jamais en production ; portaki release publie une version signée"
    )
}

#[derive(Debug, Parser)]
/// Arguments for `portaki run`.
pub struct RunArgs {
    /// The query or command to run. Bare, it lists what this module exposes.
    pub operation: Option<String>,
    /// JSON parameters.
    #[arg(long, default_value = "{}", value_name = "JSON")]
    pub params: String,
    /// `query` reads, `command` writes. Read from the manifest when omitted.
    #[arg(long, value_parser = ["query", "command"])]
    pub kind: Option<String>,
    /// Run as the host gateway (no stay) or as a guest (the fixture's stay).
    #[arg(long = "as", value_parser = ["host", "guest"], value_name = "CALLER")]
    pub caller: Option<String>,
    /// The fixture property — the default one when omitted.
    #[arg(long, value_name = "ID")]
    pub property: Option<String>,
    /// The fixture stay.
    #[arg(long, value_name = "ID")]
    pub stay: Option<String>,
    #[command(flatten)]
    pub modules: ModuleArgs,
}

#[derive(Debug, Parser)]
#[command(args_conflicts_with_subcommands = true)]
/// Arguments for `portaki scenarios`.
pub struct ScenariosArgs {
    #[command(subcommand)]
    pub action: Option<ScenariosAction>,
    #[command(flatten)]
    pub modules: ModuleArgs,
}

#[derive(Debug, Subcommand)]
pub enum ScenariosAction {
    /// Replay every surface against the seven cases, on the last build.
    Run {
        #[command(flatten)]
        modules: ModuleArgs,
    },
    /// Put the sandbox fixtures (properties, stays, guests) back as they were.
    Reset,
}

#[derive(Debug, Parser)]
/// Arguments for `portaki preview`.
pub struct PreviewArgs {
    /// The surface to render. Bare, it lists the surfaces the build declares.
    pub surface: Option<String>,
    /// The surface input, as JSON.
    #[arg(long, value_name = "JSON")]
    pub input: Option<String>,
    /// The fixture property — the default one when omitted.
    #[arg(long, value_name = "ID")]
    pub property: Option<String>,
    /// The fixture stay — none when omitted.
    #[arg(long, value_name = "ID")]
    pub stay: Option<String>,
    #[command(flatten)]
    pub modules: ModuleArgs,
}

fn open() -> Result<Platform> {
    Platform::open(&crate::profile::api_url(None))
}

/// Runs `portaki run`.
pub async fn run(args: RunArgs) -> Result<()> {
    ui::header(
        "portaki run",
        &crate::tr!(
            "Run a query or a command on the build in the hosted sandbox.",
            "Lancer une query (lire) ou une command (agir) sur le build de la sandbox hébergée."
        ),
    );
    let member = args.modules.one("run")?;
    let Some(operation) = args.operation.as_deref() else {
        return dev::list_operations(&member.root);
    };
    serde_json::from_str::<Value>(&args.params).map_err(|failure| {
        crate::exit::usage(crate::tr!(
            "--params is not JSON: {failure}",
            "--params n'est pas du JSON : {failure}"
        ))
    })?;
    let kind = match &args.kind {
        Some(kind) => kind.clone(),
        None => kind_of(&member.root, operation),
    };

    let mut platform = open()?;
    let running = ui::step(crate::tr!(
        "running {kind} {operation}",
        "exécution de {kind} {operation}"
    ));
    let body = json!({
        "operation": operation,
        "kind": kind,
        "paramsJson": args.params,
        "trigger": "manual",
        "propertyId": args.property,
        "stayId": args.stay,
        "as": args.caller,
    });
    let answer = platform
        .call(
            Method::POST,
            &format!("/dev/v1/modules/{}/dispatch", member.id),
            Some(&body),
        )
        .await
        .map_err(|failure| {
            running.abandon();
            failure.context(crate::tr!(
                "run {operation} — has portaki dev deployed {} to the sandbox?",
                "exécuter {operation} — portaki dev a-t-il déployé {} en sandbox ?",
                member.id
            ))
        })?;
    let trace: DispatchResponse =
        serde_json::from_value(answer.clone()).context("read the run's trace")?;

    match &trace.error_code {
        None => running.done(format!("{kind} {operation} — {} ms", trace.duration_ms)),
        Some(code) => {
            running.abandon();
            ui::failure(crate::tr!(
                "{kind} {operation} — refused: {code}",
                "{kind} {operation} — refusé : {code}"
            ));
        }
    }
    if ui::json() {
        ui::emit(&json!({
            "schemaVersion": 1,
            "module": member.id,
            "operation": operation,
            "kind": kind,
            "run": answer,
        }));
    } else {
        dev::print_trace(&trace);
        ui::next(&[
            (
                &format!("portaki logs --code <code>{}", flag(&args.modules)),
                &crate::tr!(
                    "follow what the module logs",
                    "suivre les journaux du module"
                ),
            ),
            (
                "portaki preview <surface>",
                &crate::tr!(
                    "render a surface of this build",
                    "vérifier le rendu d'une surface de ce build"
                ),
            ),
        ]);
        ui::blank();
    }
    match trace.error_code {
        Some(code) => anyhow::bail!(crate::tr!(
            "{operation} did not finish: {code}",
            "{operation} n'a pas abouti : {code}"
        )),
        None => Ok(()),
    }
}

/// `command` when the manifest files the operation under the commands, `query` otherwise — the
/// platform's default.
fn kind_of(module_root: &std::path::Path, operation: &str) -> String {
    let command = crate::manifest::load_manifest(module_root, None)
        .map(|(manifest, _)| manifest.commands.iter().any(|c| c.name == operation))
        .unwrap_or(false);
    if command { "command" } else { "query" }.to_string()
}

/// `--module <id>` to carry over into the next command, when it was given.
fn flag(modules: &ModuleArgs) -> String {
    modules
        .module
        .as_deref()
        .map(|id| format!(" --module {id}"))
        .unwrap_or_default()
}

/// Runs `portaki scenarios`.
pub async fn scenarios(args: ScenariosArgs) -> Result<()> {
    ui::header(
        "portaki scenarios",
        &crate::tr!("Every surface against the seven pathological stays — the Scenarios tab of the sandbox.", "Chaque surface face aux sept séjours pathologiques — l'onglet Scénarios de la sandbox."),
    );
    let mut platform = open()?;
    let (modules, replay) = match args.action {
        Some(ScenariosAction::Reset) => return reset(&mut platform).await,
        Some(ScenariosAction::Run { modules }) => (modules, true),
        None => (args.modules, false),
    };
    let member = modules.one("scenarios")?;
    let path = format!("/dev/v1/modules/{}/scenarios", member.id);
    let answer = if replay {
        let running = ui::step(&crate::tr!(
            "replaying the 7 scenarios",
            "relance des 7 scénarios"
        ));
        let answer = platform
            .call(Method::POST, &format!("{path}/run"), None)
            .await
            .map_err(|failure| {
                running.abandon();
                failure
            })?;
        running.done(&crate::tr!("replayed", "relancés"));
        answer
    } else {
        platform
            .get(&path)
            .await?
            .unwrap_or(Value::Array(Vec::new()))
    };
    let cells: Vec<ScenarioCell> =
        serde_json::from_value(answer).context("read the scenario grid")?;
    let failing = cells.iter().filter(|cell| cell.status == "fail").count();

    if ui::json() {
        ui::emit(&json!({ "schemaVersion": 1, "module": member.id, "cells": cells }));
    } else if cells.is_empty() {
        ui::skipped(&crate::tr!(
            "no scenario played yet on this build",
            "aucun scénario joué sur ce build"
        ));
        ui::next(&[(
            &format!("portaki scenarios run{}", flag(&modules)),
            &crate::tr!(
                "replay the seven cases now",
                "relancer les sept cas maintenant"
            ),
        )]);
        ui::blank();
    } else {
        ui::success(dev::scenarios_line(&cells));
        dev::print_grid(&cells);
        if failing > 0 {
            ui::next(&[(
                &format!("portaki preview <surface>{}", flag(&modules)),
                &crate::tr!(
                    "render the failing surface and fix it",
                    "vérifier le rendu de la surface en échec, et la corriger"
                ),
            )]);
        } else {
            ui::next(&[(
                &format!("portaki check{}", flag(&modules)),
                &crate::tr!(
                    "the gate portaki release applies",
                    "la porte que portaki release applique"
                ),
            )]);
        }
        ui::blank();
    }
    if failing > 0 {
        anyhow::bail!(crate::tr!(
            "{failing} scenario(s) fail",
            "{failing} scénario(s) en échec"
        ));
    }
    Ok(())
}

async fn reset(platform: &mut Platform) -> Result<()> {
    let resetting = ui::step(&crate::tr!(
        "resetting the sandbox fixtures",
        "réinitialisation des fixtures de la sandbox"
    ));
    let answer = platform
        .call(Method::POST, "/dev/v1/sandbox/fixtures/reset", None)
        .await
        .map_err(|failure| {
            resetting.abandon();
            failure
        })?;
    let generation = answer["generation"].as_u64().unwrap_or(0);
    if ui::json() {
        ui::emit(&json!({ "schemaVersion": 1, "generation": generation }));
        return Ok(());
    }
    resetting.done(crate::tr!(
        "fixtures reset (generation {generation})",
        "fixtures réinitialisées (génération {generation})"
    ));
    ui::next(&[(
        "portaki scenarios run",
        &crate::tr!(
            "replay the seven cases on fresh fixtures",
            "relancer les sept cas sur des fixtures neuves"
        ),
    )]);
    ui::blank();
    Ok(())
}

/// Runs `portaki preview`.
pub async fn preview(args: PreviewArgs) -> Result<()> {
    ui::header(
        "portaki preview",
        &crate::tr!("Render a surface of the sandbox build, as the host or the guest will see it.", "Vérifier le rendu d'une surface du build de sandbox, là où l'hôte ou le voyageur le verra."),
    );
    let member = args.modules.one("preview")?;
    let mut platform = open()?;
    let Some(surface) = args.surface.as_deref() else {
        return surfaces(&mut platform, &member.id, &args.modules).await;
    };
    if let Some(input) = &args.input {
        serde_json::from_str::<Value>(input).map_err(|failure| {
            crate::exit::usage(crate::tr!(
                "--input is not JSON: {failure}",
                "--input n'est pas du JSON : {failure}"
            ))
        })?;
    }
    // Before the render, on stderr: a script reading `--json` is not bothered by it, and nobody
    // takes a preview that worked for a version that is ready.
    ui::warn(sandbox_only());
    let rendering = ui::step(crate::tr!("rendering {surface}", "rendu de {surface}"));
    let body = json!({
        "inputJson": args.input,
        "propertyId": args.property,
        "stayId": args.stay,
    });
    let answer = platform
        .call(
            Method::POST,
            &format!("/dev/v1/modules/{}/surfaces/{surface}/render", member.id),
            Some(&body),
        )
        .await
        .map_err(|failure| {
            rendering.abandon();
            failure.context(crate::tr!(
                "render {surface} — has portaki dev deployed {} to the sandbox?",
                "rendre {surface} — portaki dev a-t-il déployé {} en sandbox ?",
                member.id
            ))
        })?;
    let rendered = answer["rendered"].as_bool().unwrap_or(false);
    let code = answer["errorCode"].as_str().map(str::to_string);

    if rendered {
        rendering.done(crate::tr!(
            "{surface} rendered · {}",
            "{surface} rendue · {}",
            answer["type"].as_str().unwrap_or_default()
        ));
    } else {
        rendering.abandon();
    }
    if ui::json() {
        ui::emit(&json!({ "schemaVersion": 1, "module": member.id, "preview": answer }));
    } else {
        if let Some(tree) = answer["tree"].as_str() {
            let pretty = serde_json::from_str::<Value>(tree)
                .and_then(|tree| serde_json::to_string_pretty(&tree))
                .unwrap_or_else(|_| tree.to_string());
            ui::result(pretty);
        }
        ui::next(&[(
            &format!("portaki scenarios run{}", flag(&args.modules)),
            &crate::tr!(
                "the same surface against the seven pathological stays",
                "la même surface face aux sept séjours pathologiques"
            ),
        )]);
        ui::blank();
    }
    if rendered {
        Ok(())
    } else {
        anyhow::bail!(crate::tr!(
            "{surface} did not render: {}",
            "{surface} ne s'est pas rendue : {}",
            code.unwrap_or_else(|| "no tree".to_string())
        ))
    }
}

/// The surfaces the build declares, and how to render one.
async fn surfaces(platform: &mut Platform, id: &str, modules: &ModuleArgs) -> Result<()> {
    let declared = platform
        .get(&format!("/dev/v1/modules/{id}/surfaces"))
        .await?
        .unwrap_or(Value::Array(Vec::new()));
    if ui::json() {
        ui::emit(&json!({ "schemaVersion": 1, "module": id, "surfaces": declared }));
        return Ok(());
    }
    let rows: Vec<(String, String)> = declared
        .as_array()
        .into_iter()
        .flatten()
        .map(|surface| {
            let types: Vec<&str> = surface["types"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .collect();
            let audience = if surface["guest"].as_bool().unwrap_or(false) {
                "guest"
            } else {
                "host"
            };
            (
                surface["id"].as_str().unwrap_or_default().to_string(),
                format!("{audience} · {}", types.join(", ")),
            )
        })
        .collect();
    let Some((sample, _)) = rows.first() else {
        ui::skipped(crate::tr!(
            "{id} has no build in the sandbox, or declares no surface",
            "{id} n'a aucun build en sandbox, ou ne déclare aucune surface"
        ));
        ui::next(&[(
            "portaki dev",
            &crate::tr!(
                "build and deploy to the sandbox",
                "compiler et déployer en sandbox"
            ),
        )]);
        ui::blank();
        return Ok(());
    };
    let borrowed: Vec<(&str, &str)> = rows.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    ui::list("surfaces", &borrowed);
    ui::next(&[(
        &format!("portaki preview {sample}{}", flag(modules)),
        &crate::tr!("render it", "vérifier son rendu"),
    )]);
    ui::blank();
    Ok(())
}
