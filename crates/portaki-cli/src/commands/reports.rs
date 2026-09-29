//! `portaki reports` — the developer space's Reports page, in the terminal.
//!
//! What comes back about a module: the errors Portaki catches at run time, the problems and
//! suggestions from hosts. You do not answer the host: you fix, then mark the report resolved with
//! an internal note. Routes: `GET /dev/v1/modules/{id}/reports` and `PATCH /dev/v1/reports/{id}`.

use anyhow::Result;
use clap::{Parser, Subcommand};
use reqwest::Method;
use serde_json::{json, Value};

use crate::api::Platform;
use crate::ui;
use crate::workspace::ModuleArgs;

/// One page is enough for a terminal; beyond that, the developer space paginates.
const PAGE_SIZE: u32 = 50;

#[derive(Debug, Parser)]
#[command(args_conflicts_with_subcommands = true)]
/// Arguments for `portaki reports`.
pub struct ReportsArgs {
    #[command(subcommand)]
    pub action: Option<ReportsAction>,
    /// Only the reports still open.
    #[arg(long)]
    pub open: bool,
    /// Only one type: `error`, `problem` or `suggestion`.
    #[arg(long = "type", value_name = "TYPE")]
    pub kind: Option<String>,
    #[command(flatten)]
    pub modules: ModuleArgs,
}

#[derive(Debug, Subcommand)]
pub enum ReportsAction {
    /// Mark a report resolved, with an internal note — never shown to the host.
    Resolve {
        /// The report id, as `portaki reports` lists it.
        id: String,
        /// Why it is resolved: what was fixed, and in which version.
        #[arg(long, required = true)]
        note: String,
    },
}

/// Runs `portaki reports`.
pub async fn run(args: ReportsArgs) -> Result<()> {
    ui::header(
        "portaki reports",
        &crate::tr!("Errors Portaki caught, problems and suggestions from hosts — fix, then resolve.", "Les erreurs relevées par Portaki, les problèmes et suggestions des hôtes — corrigez, puis marquez résolu."),
    );
    let mut platform = Platform::open(&crate::profile::api_url(None))?;
    if let Some(ReportsAction::Resolve { id, note }) = args.action {
        return resolve(&mut platform, &id, &note).await;
    }

    let mut query = format!("?page=0&size={PAGE_SIZE}");
    if args.open {
        query.push_str("&status=open");
    }
    if let Some(kind) = &args.kind {
        query.push_str(&format!("&type={kind}"));
    }
    let mut modules = Vec::new();
    let mut open = 0;
    for member in args.modules.resolve()? {
        let page = platform
            .get(&format!("/dev/v1/modules/{}/reports{query}", member.id))
            .await?
            .unwrap_or(Value::Null);
        open += page["counts"]["open"].as_u64().unwrap_or(0);
        if !ui::json() {
            show(&member.id, &page);
        }
        modules.push(json!({ "id": member.id, "reports": page }));
    }

    if ui::json() {
        ui::emit(&json!({ "schemaVersion": 1, "modules": modules }));
        return Ok(());
    }
    if open > 0 {
        ui::next(&[(
            "portaki reports resolve <id> --note \"…\"",
            &crate::tr!(
                "once fixed: resolve it, with an internal note",
                "une fois corrigé : le marquer résolu, avec une note interne"
            ),
        )]);
    }
    ui::blank();
    Ok(())
}

fn show(id: &str, page: &Value) {
    ui::section(id);
    let items = page["items"].as_array().cloned().unwrap_or_default();
    if items.is_empty() {
        ui::skipped(crate::tr!("no report", "aucun rapport"));
        return;
    }
    for report in &items {
        let text = |key: &str| report[key].as_str().unwrap_or_default().to_string();
        let count = report["count"]
            .as_u64()
            .filter(|count| *count > 1)
            .map(|count| format!(" ×{count}"))
            .unwrap_or_default();
        let line = format!(
            "{} · {}{count}  {}",
            text("type"),
            text("title"),
            text("id")
        );
        match text("status").as_str() {
            "open" if text("type") == "error" => ui::failure(line),
            "open" => ui::warn(line),
            status => ui::skipped(format!("{line} ({status})")),
        }
        if let Some(surface) = report["surface"].as_str() {
            ui::detail(format!("surface {surface}"));
        }
        if let Some(note) = report["internalNote"].as_str() {
            ui::detail(crate::tr!("note: {note}", "note : {note}"));
        }
    }
    let total = page["total"].as_u64().unwrap_or(0);
    if total > items.len() as u64 {
        ui::detail(crate::tr!(
            "{} more in the developer space",
            "{} de plus dans l'espace développeur",
            total - items.len() as u64
        ));
    }
}

async fn resolve(platform: &mut Platform, id: &str, note: &str) -> Result<()> {
    if note.trim().is_empty() {
        return Err(crate::exit::usage(&crate::tr!(
            "--note: say what was fixed — the note stays internal, the host never reads it",
            "--note : dites ce qui a été corrigé — la note reste interne, l'hôte ne la lit jamais"
        )));
    }
    let resolving = ui::step(crate::tr!("resolving {id}", "résolution de {id}"));
    let report = platform
        .call(
            Method::PATCH,
            &format!("/dev/v1/reports/{id}"),
            Some(&json!({ "status": "resolved", "internalNote": note })),
        )
        .await
        .map_err(|failure| {
            resolving.abandon();
            failure
        })?;
    resolving.done(crate::tr!(
        "{} resolved",
        "{} résolu",
        report["title"].as_str().unwrap_or(id)
    ));
    if ui::json() {
        ui::emit(&json!({ "schemaVersion": 1, "report": report }));
    } else {
        ui::next(&[(
            "portaki reports --open",
            &crate::tr!("what is still open", "ce qui reste ouvert"),
        )]);
        ui::blank();
    }
    Ok(())
}
