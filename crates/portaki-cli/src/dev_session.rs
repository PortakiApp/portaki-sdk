//! One `portaki dev` session per account, held at devapi.
//!
//! # Why on the platform side
//!
//! The [`crate::watch_lock`] lock only sees its own machine. Two laptops, one account, one
//! sandbox: the two sessions push different modules in turn and each undoes what the other has
//! just done. Only the server sees both.
//!
//! # Why devapi and not the registry
//!
//! The lease first lived at the registry, where it could only be asked for politely: the
//! `dev-deploy` it protects happens at devapi, which did not see it. A client that ignored it —
//! or that had lost the registry when taking it — overwrote the other one's sandbox all the same.
//! From devapi, the refusal is raised against the write itself.
//!
//! # A lease, not a lock
//!
//! Nothing here can interrogate a remote process. A laptop that gets closed, a network that gets
//! cut, a `kill -9`: the holder disappears without handing anything back, and the account would
//! stay taken until a human stepped in. The server therefore grants a lease with an expiry, which
//! this session pushes back as long as it lives — and which frees the account on its own when it
//! stops.
//!
//! # What a network outage does
//!
//! **When taking it**: we warn and carry on without a lease. Refusing to develop because a
//! locking service does not answer would cost more than the annoyance it avoids — and the local
//! lock still covers this machine.
//!
//! **When renewing**: we retry silently. The lease lasts several times the interval, so a passing
//! outage costs nothing. Past the expiry, we say so — the account may already have been taken
//! over elsewhere.
//!
//! **Taken over by someone else**: there we stop. Carrying on would be exactly the mutual
//! overwriting this lease exists to prevent.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};

use crate::ui;

/// What the server returns when it grants the lease.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Lease {
    /// How long the server wants us to wait before pushing the lease back. Returned by it rather
    /// than guessed here: the TTL belongs to it, and a client that guessed it wrong would lose
    /// its session without having done anything wrong.
    renew_after_seconds: u64,
}

/// What it returns when it refuses it.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Held {
    module_id: String,
    machine: String,
}

/// The session under way: the local lock, and the account lease if it could be taken.
pub struct DevSession {
    local: crate::watch_lock::WatchLock,
    lease: Option<Arc<LeaseHolder>>,
}

/// What it takes to give the place back from elsewhere — the task watching for ctrl-c, notably.
///
/// A light handle, and not the session itself: a task that held on to the session would keep its
/// `Drop` from running on the normal return, and the local lock would outlive every failure.
#[derive(Clone)]
pub struct Release {
    lock: std::path::PathBuf,
    lease: Option<Arc<LeaseHolder>>,
}

impl Release {
    /// Gives back the local lock, then the lease. Best effort: we are leaving either way.
    pub async fn now(&self) {
        let _ = std::fs::remove_file(&self.lock);
        let Some(holder) = &self.lease else {
            return;
        };
        let _ = crate::http::client()
            .delete(format!("{}/dev/v1/dev-watch", holder.base_url))
            .query(&[("sessionId", &holder.session_id)])
            .bearer_auth(holder.token())
            .timeout(Duration::from_secs(3))
            .send()
            .await;
    }
}

struct LeaseHolder {
    base_url: String,
    /// Where to renew the token — the authentication platform, not necessarily devapi.
    auth_url: String,
    session_id: String,
    /// Renewed in place: the lease lives far longer than a token's fifteen minutes, and the one
    /// from start-up made every renewal past that delay fail — silently.
    token: std::sync::Mutex<String>,
}

impl LeaseHolder {
    fn token(&self) -> String {
        self.token
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

/// Takes the place — locally first, then at devapi.
///
/// The local lock first because it is immediate and needs no network: no point going to ask the
/// server only to be refused by one's own machine.
pub async fn start(
    base_url: &str,
    auth_url: &str,
    module_id: &str,
    token: &str,
) -> Result<DevSession> {
    let local = crate::watch_lock::acquire(module_id)?;
    let session_id = uuid::Uuid::new_v4().to_string();

    let holder = Arc::new(LeaseHolder {
        base_url: base_url.trim_end_matches('/').to_string(),
        auth_url: auth_url.to_string(),
        session_id,
        token: std::sync::Mutex::new(token.to_string()),
    });

    match hold(&holder, module_id).await {
        Ok(Kept::Ours(lease)) => {
            spawn_renewal(Arc::clone(&holder), module_id.to_string(), lease);
            Ok(DevSession {
                local,
                lease: Some(holder),
            })
        }
        Ok(Kept::Theirs(held)) => anyhow::bail!(
            "your account already has a portaki dev session on {} ({}) — stop it there, or \
             wait: one that ended without handing it back frees itself shortly",
            held.module_id,
            held.machine
        ),
        Err(unreachable) => {
            // devapi is not answering. We say so and carry on: the local lock still covers this
            // machine, and refusing to work for that reason would cost more than the annoyance
            // it avoids. The server will refuse the deploy anyway if someone else holds the
            // place — it is the one guarding the sandbox, not us.
            ui::warn("could not reach the dev platform — this session is not held account-wide");
            // The whole chain: the context alone says what was attempted, not what failed.
            ui::detail(format!("{unreachable:#}"));
            ui::advice("another machine on this account could start one too");
            Ok(DevSession { local, lease: None })
        }
    }
}

impl DevSession {
    /// The session as the lease knows it, if we obtained it.
    ///
    /// `None` when taking it failed: we then push with nothing to prove, and it is the server
    /// that decides — it only admits an anonymous push if nobody else holds the place. Sending a
    /// made-up id would be worse, it would look like a holder.
    pub fn session_id(&self) -> Option<&str> {
        self.lease.as_ref().map(|holder| holder.session_id.as_str())
    }

    /// What it takes to give the place back from another task.
    pub fn release(&self) -> Release {
        Release {
            lock: self.local.path().to_path_buf(),
            lease: self.lease.clone(),
        }
    }
}

enum Kept {
    Ours(Lease),
    Theirs(Held),
}

/// A 401 is not a refusal: the token has expired. We renew it once and ask again — without which
/// a token that was already stale at start-up read as "platform unreachable".
async fn hold(holder: &LeaseHolder, module_id: &str) -> Result<Kept> {
    let stale = holder.token();
    let mut response = request_hold(holder, module_id, &stale).await?;
    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        let renewed = crate::auth::refresh(&holder.auth_url, &stale).await?;
        *holder
            .token
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = renewed.clone();
        response = request_hold(holder, module_id, &renewed).await?;
    }

    if response.status() == reqwest::StatusCode::CONFLICT {
        return Ok(Kept::Theirs(
            response
                .json()
                .await
                .context("read who holds the session")?,
        ));
    }
    let status = response.status();
    if !status.is_success() {
        anyhow::bail!("the dev platform answered {status}");
    }
    Ok(Kept::Ours(response.json().await.context("read the lease")?))
}

async fn request_hold(
    holder: &LeaseHolder,
    module_id: &str,
    token: &str,
) -> Result<reqwest::Response> {
    crate::http::client()
        .put(format!("{}/dev/v1/dev-watch", holder.base_url))
        .bearer_auth(token)
        .json(&serde_json::json!({
            "sessionId": holder.session_id,
            "moduleId": module_id,
            "machine": machine(),
        }))
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .context("ask the dev platform for the watch session")
}

/// Pushes the lease back as long as the session lives.
fn spawn_renewal(holder: Arc<LeaseHolder>, module_id: String, first: Lease) {
    tokio::spawn(async move {
        let every = Duration::from_secs(first.renew_after_seconds.clamp(5, 600));
        // The lease lasts several times this interval: a few failures in a row put nothing at
        // risk, and keeping quiet avoids worrying anyone over a one-second outage.
        let mut missed = 0_u32;
        loop {
            tokio::time::sleep(every).await;
            match hold(&holder, &module_id).await {
                Ok(Kept::Ours(_)) => missed = 0,
                Ok(Kept::Theirs(held)) => {
                    // The lease has been taken over. Carrying on means overwriting the other
                    // session's work — precisely what all of this exists to prevent.
                    ui::blank();
                    ui::failure(format!(
                        "this account's portaki dev session was taken over by {} ({})",
                        held.module_id, held.machine
                    ));
                    ui::detail("stopping — two sessions would undo each other's deploys");
                    std::process::exit(1);
                }
                Err(_) => {
                    missed += 1;
                    // Three failures: we are past the expiry, the account may be free — or
                    // already taken over elsewhere.
                    if missed == 3 {
                        ui::blank();
                        ui::warn("the dev platform has not answered for a while");
                        ui::detail("this session may no longer be held account-wide");
                    }
                }
            }
        }
    });
}

/// The machine's name, to place a session held elsewhere.
fn machine() -> String {
    for variable in ["HOSTNAME", "HOST", "COMPUTERNAME"] {
        if let Ok(name) = std::env::var(variable) {
            if !name.trim().is_empty() {
                return name.trim().to_string();
            }
        }
    }
    std::process::Command::new("hostname")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The name serves to place a session held elsewhere: it must always say something.
    #[test]
    fn the_machine_always_has_a_name() {
        assert!(!machine().is_empty());
    }

    /// The server sets the pace, but an absurd value must produce neither a tight loop nor a
    /// renewal that never comes.
    #[test]
    fn a_nonsensical_interval_is_brought_back_to_reason() {
        assert_eq!(0_u64.clamp(5, 600), 5);
        assert_eq!(86_400_u64.clamp(5, 600), 600);
        assert_eq!(30_u64.clamp(5, 600), 30);
    }

    #[test]
    fn the_server_answer_reads_as_the_cli_expects() {
        let lease: Lease =
            serde_json::from_value(serde_json::json!({ "renewAfterSeconds": 30 })).unwrap();
        assert_eq!(lease.renew_after_seconds, 30);

        let held: Held = serde_json::from_value(
            serde_json::json!({ "moduleId": "weather", "machine": "laptop" }),
        )
        .unwrap();
        assert_eq!(held.module_id, "weather");
    }
}
