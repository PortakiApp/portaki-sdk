//! `portaki logs` — the sandbox's live log stream, in the terminal.
//!
//! The same stream the sandbox dock's Journaux tab reads: `GET /dev/v1/modules/{id}/logs`, in
//! server-sent events, one `{ ts, level, src, msg }` per event. `portaki dev --watch` follows it
//! too, next to its build output.

use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;

use crate::commands::dev::{self, Unauthorized};
use crate::{auth, ui};

#[derive(Debug, Parser)]
/// Arguments for `portaki logs`.
pub struct LogsArgs {
    /// The module whose logs to follow. Defaults to the module of the current directory.
    pub module: Option<String>,
    /// Only the lines that mention this error code (`connector_timeout`, `missing_field`…).
    #[arg(long)]
    pub code: Option<String>,
    /// Base URL of the dev platform (defaults like `portaki dev`).
    #[arg(long)]
    pub url: Option<String>,
}

/// One log line, as devapi streams it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub(crate) struct LogLine {
    #[serde(default)]
    pub ts: String,
    #[serde(default)]
    pub level: String,
    #[serde(default)]
    pub src: String,
    #[serde(default)]
    pub msg: String,
}

/// The stream is not served here — a platform older than the dock.
#[derive(Debug)]
pub(crate) struct NotServed;

impl std::fmt::Display for NotServed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("this platform streams no logs")
    }
}

impl std::error::Error for NotServed {}

/// Runs `portaki logs`.
pub async fn run(args: LogsArgs) -> Result<()> {
    ui::header(
        "portaki logs",
        "What the module logs in the sandbox, as it happens — the last hour is kept.",
    );
    let module_id = match args.module {
        Some(id) => id,
        None => dev::read_module_id(&std::env::current_dir().context("current_dir")?)?,
    };
    let base = dev::resolve_base_url(
        args.url.as_deref(),
        std::env::var("PORTAKI_DEV_URL").ok().as_deref(),
        std::env::var("PORTAKI_API_URL").ok().as_deref(),
    );
    let auth_url = auth::api_base_url(args.url.as_deref());
    let mut token = auth::access_token()?;
    ui::field("module", &module_id);
    if let Some(code) = &args.code {
        ui::field("code", code);
    }
    ui::advice("ctrl-c to stop");
    ui::blank();

    let code = args.code.as_deref();
    let show = |line: LogLine| {
        if code.map_or(true, |code| mentions(&line, code)) {
            print(&line);
        }
    };
    match follow(&base, &module_id, &token, show).await {
        Err(failure) if failure.is::<Unauthorized>() => {
            token = dev::renew(&auth_url, &token).await?;
            follow(&base, &module_id, &token, show).await
        }
        other => other,
    }
}

/// Keeps the stream open for a whole `--watch` session: a dropped connection comes back, an
/// expired token is renewed. Only a platform without the stream ends it.
pub(crate) async fn follow_forever(
    base: String,
    auth_url: String,
    module_id: String,
    mut token: String,
) {
    loop {
        match follow(&base, &module_id, &token, |line| print(&line)).await {
            Err(failure) if failure.is::<NotServed>() => return,
            Err(failure) if failure.is::<Unauthorized>() => {
                if let Ok(renewed) = dev::renew(&auth_url, &token).await {
                    token = renewed;
                    continue;
                }
            }
            _ => {}
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

/// Reads the stream until the platform closes it, handing each line over.
async fn follow(
    base: &str,
    module_id: &str,
    token: &str,
    mut on_line: impl FnMut(LogLine),
) -> Result<()> {
    // No overall deadline: a stream lasts as long as the session. The connection one stays.
    let mut response =
        crate::http::client_with(crate::http::CONNECT, Duration::from_secs(24 * 3600))
            .get(format!("{base}/dev/v1/modules/{module_id}/logs"))
            .bearer_auth(token)
            .header("Accept", "text/event-stream")
            .send()
            .await
            .context("open the log stream")?;
    match response.status().as_u16() {
        401 => return Err(anyhow::Error::new(Unauthorized)),
        404 => return Err(anyhow::Error::new(NotServed)),
        status if !(200..300).contains(&status) => {
            anyhow::bail!("the dev platform answered {status} to the log stream")
        }
        _ => {}
    }
    let mut buffer = String::new();
    while let Some(chunk) = response.chunk().await.context("read the log stream")? {
        buffer.push_str(&String::from_utf8_lossy(&chunk));
        for data in take_events(&mut buffer) {
            if let Ok(line) = serde_json::from_str::<LogLine>(&data) {
                on_line(line);
            }
        }
    }
    Ok(())
}

/// The `data` of every complete event in `buffer`, which keeps what is not complete yet.
fn take_events(buffer: &mut String) -> Vec<String> {
    if buffer.contains('\r') {
        *buffer = buffer.replace("\r\n", "\n");
    }
    let mut events = Vec::new();
    while let Some(end) = buffer.find("\n\n") {
        let event: String = buffer.drain(..end + 2).collect();
        let data: Vec<&str> = event
            .lines()
            .filter_map(|line| line.strip_prefix("data:"))
            .map(|data| data.strip_prefix(' ').unwrap_or(data))
            .collect();
        if !data.is_empty() {
            events.push(data.join("\n"));
        }
    }
    events
}

fn mentions(line: &LogLine, code: &str) -> bool {
    line.msg.contains(code) || line.src.contains(code)
}

pub(crate) fn print(line: &LogLine) {
    let at = chrono::DateTime::parse_from_rfc3339(&line.ts)
        .map(|at| {
            at.with_timezone(&chrono::Local)
                .format("%H:%M:%S")
                .to_string()
        })
        .unwrap_or_else(|_| line.ts.clone());
    let text = format!("{at}  {:<5}  {}  {}", line.level, line.src, line.msg);
    match line.level.to_ascii_lowercase().as_str() {
        "error" => ui::failure(text),
        "warn" | "warning" => ui::warn(text),
        _ => ui::detail(text),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_events_are_taken_and_the_rest_kept() {
        let mut buffer = String::from(
            "data: {\"msg\":\"a\"}\n\n: keep-alive\n\nevent: log\r\ndata: {\"msg\":\"b\"}\r\n\r\ndata: {\"ms",
        );

        let events = take_events(&mut buffer);

        assert_eq!(events, vec![r#"{"msg":"a"}"#, r#"{"msg":"b"}"#]);
        assert_eq!(buffer, "data: {\"ms");
    }

    #[test]
    fn a_line_is_read_as_devapi_writes_it() {
        let line: LogLine = serde_json::from_str(
            r#"{"ts":"2026-09-25T10:00:00Z","level":"error","src":"host","msg":"connector_timeout nuki"}"#,
        )
        .unwrap();

        assert!(mentions(&line, "connector_timeout"));
        assert!(!mentions(&line, "missing_field"));
    }
}
