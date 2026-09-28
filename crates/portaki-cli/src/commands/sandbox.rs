//! `portaki run`, `portaki scenarios`, `portaki preview` — la page Sandbox de l'espace
//! développeur, dans le terminal.
//!
//! Toutes trois agissent sur le dernier build que `portaki dev` a poussé, par les routes que
//! l'espace développeur appelle déjà : `POST /dev/v1/modules/{id}/dispatch`,
//! `GET|POST /dev/v1/modules/{id}/scenarios[/run]`, `POST /dev/v1/sandbox/fixtures/reset`,
//! `GET /dev/v1/modules/{id}/surfaces` et `POST …/surfaces/{surface}/render`. Aucune ne compile
//! ni ne déploie : c'est `portaki dev` qui le fait.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use reqwest::Method;
use serde_json::{json, Value};

use crate::api::Platform;
use crate::commands::dev::{self, DispatchResponse, ScenarioCell};
use crate::ui;
use crate::workspace::ModuleArgs;

/// Ce que devient un build de sandbox hors d'elle : rien. Dit à chaque rendu, parce qu'un aperçu
/// réussi ressemble à s'y méprendre à une version prête.
pub const SANDBOX_ONLY: &str =
    "sandbox build, unsigned — it will never run in production; portaki release publishes a signed version";

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
        "Run a query or a command on the build in the hosted sandbox.",
    );
    let member = args.modules.one("run")?;
    let Some(operation) = args.operation.as_deref() else {
        return dev::list_operations(&member.root);
    };
    serde_json::from_str::<Value>(&args.params)
        .map_err(|failure| crate::exit::usage(format!("--params is not JSON: {failure}")))?;
    let kind = match &args.kind {
        Some(kind) => kind.clone(),
        None => kind_of(&member.root, operation),
    };

    let mut platform = open()?;
    let running = ui::step(format!("running {kind} {operation}"));
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
            failure.context(format!(
                "run {operation} — has portaki dev deployed {} to the sandbox?",
                member.id
            ))
        })?;
    let trace: DispatchResponse =
        serde_json::from_value(answer.clone()).context("read the run's trace")?;

    match &trace.error_code {
        None => running.done(format!("{kind} {operation} — {} ms", trace.duration_ms)),
        Some(code) => {
            running.abandon();
            ui::failure(format!("{kind} {operation} — refused: {code}"));
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
                "follow what the module logs",
            ),
            (
                "portaki preview <surface>",
                "render a surface of this build",
            ),
        ]);
        ui::blank();
    }
    match trace.error_code {
        Some(code) => anyhow::bail!("{operation} did not finish: {code}"),
        None => Ok(()),
    }
}

/// `command` quand le manifeste range l'opération parmi les commandes, `query` sinon — le défaut
/// de la plateforme.
fn kind_of(module_root: &std::path::Path, operation: &str) -> String {
    let command = crate::manifest::load_manifest(module_root, None)
        .map(|(manifest, _)| manifest.commands.iter().any(|c| c.name == operation))
        .unwrap_or(false);
    if command { "command" } else { "query" }.to_string()
}

/// `--module <id>` à reporter dans la commande suivante, quand il a été donné.
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
        "Every surface against the seven pathological stays — the Scenarios tab of the sandbox.",
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
        let running = ui::step("replaying the 7 scenarios");
        let answer = platform
            .call(Method::POST, &format!("{path}/run"), None)
            .await
            .map_err(|failure| {
                running.abandon();
                failure
            })?;
        running.done("replayed");
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
        ui::skipped("no scenario played yet on this build");
        ui::next(&[(
            &format!("portaki scenarios run{}", flag(&modules)),
            "replay the seven cases now",
        )]);
        ui::blank();
    } else {
        ui::success(dev::scenarios_line(&cells));
        dev::print_grid(&cells);
        if failing > 0 {
            ui::next(&[(
                &format!("portaki preview <surface>{}", flag(&modules)),
                "render the failing surface and fix it",
            )]);
        } else {
            ui::next(&[(
                &format!("portaki check{}", flag(&modules)),
                "the gate portaki release applies",
            )]);
        }
        ui::blank();
    }
    if failing > 0 {
        anyhow::bail!("{failing} scenario(s) fail");
    }
    Ok(())
}

async fn reset(platform: &mut Platform) -> Result<()> {
    let resetting = ui::step("resetting the sandbox fixtures");
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
    resetting.done(format!("fixtures reset (generation {generation})"));
    ui::next(&[(
        "portaki scenarios run",
        "replay the seven cases on fresh fixtures",
    )]);
    ui::blank();
    Ok(())
}

/// Runs `portaki preview`.
pub async fn preview(args: PreviewArgs) -> Result<()> {
    ui::header(
        "portaki preview",
        "Render a surface of the sandbox build, as the host or the guest will see it.",
    );
    let member = args.modules.one("preview")?;
    let mut platform = open()?;
    let Some(surface) = args.surface.as_deref() else {
        return surfaces(&mut platform, &member.id, &args.modules).await;
    };
    if let Some(input) = &args.input {
        serde_json::from_str::<Value>(input)
            .map_err(|failure| crate::exit::usage(format!("--input is not JSON: {failure}")))?;
    }
    // Avant le rendu, sur stderr : un script qui lit `--json` n'en est pas gêné, et personne ne
    // prend un aperçu réussi pour une version prête.
    ui::warn(SANDBOX_ONLY);
    let rendering = ui::step(format!("rendering {surface}"));
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
            failure.context(format!(
                "render {surface} — has portaki dev deployed {} to the sandbox?",
                member.id
            ))
        })?;
    let rendered = answer["rendered"].as_bool().unwrap_or(false);
    let code = answer["errorCode"].as_str().map(str::to_string);

    if rendered {
        rendering.done(format!(
            "{surface} rendered · {}",
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
            "the same surface against the seven pathological stays",
        )]);
        ui::blank();
    }
    if rendered {
        Ok(())
    } else {
        anyhow::bail!(
            "{surface} did not render: {}",
            code.unwrap_or_else(|| "no tree".to_string())
        )
    }
}

/// Les surfaces que le build déclare, et comment en rendre une.
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
        ui::skipped(format!(
            "{id} has no build in the sandbox, or declares no surface"
        ));
        ui::next(&[("portaki dev", "build and deploy to the sandbox")]);
        ui::blank();
        return Ok(());
    };
    let borrowed: Vec<(&str, &str)> = rows.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    ui::list("surfaces", &borrowed);
    ui::next(&[(
        &format!("portaki preview {sample}{}", flag(modules)),
        "render it",
    )]);
    ui::blank();
    Ok(())
}
