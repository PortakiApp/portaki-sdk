//! Une seule session `--watch` par compte, tenue auprès du registre.
//!
//! # Pourquoi côté registre
//!
//! Le verrou de [`crate::watch_lock`] ne voit que sa machine. Deux portables, un seul compte,
//! un seul bac à sable : les deux sessions poussent tour à tour des modules différents et
//! chacune défait ce que l'autre vient de faire. Seul le registre voit les deux.
//!
//! # Un bail, pas un verrou
//!
//! Rien ici ne peut interroger un processus distant. Un portable qu'on ferme, un réseau qu'on
//! coupe, un `kill -9` : le détenteur disparaît sans rien rendre, et le compte resterait pris
//! jusqu'à intervention humaine. Le registre donne donc un bail à échéance, que cette session
//! repousse tant qu'elle vit — et qui libère le compte tout seul quand elle cesse.
//!
//! # Ce que fait un réseau qui tombe
//!
//! **À la prise** : on prévient et on continue sans bail. Refuser de développer parce qu'un
//! service de verrouillage ne répond pas coûterait plus que la gêne qu'il évite — et le verrou
//! local couvre encore cette machine.
//!
//! **Au renouvellement** : on réessaie en silence. Le bail dure plusieurs fois l'intervalle,
//! donc une coupure passagère ne coûte rien. Passé l'échéance, on le dit — le compte est
//! peut-être déjà repris ailleurs.
//!
//! **Repris par quelqu'un d'autre** : là on s'arrête. Continuer serait exactement l'écrasement
//! mutuel que ce bail existe pour empêcher.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};

use crate::ui;

/// Ce que le registre rend quand il accorde le bail.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Lease {
    /// Ce que le serveur veut qu'on attende avant de repousser. Rendu par lui plutôt que deviné
    /// ici : le TTL lui appartient, et un client qui le devine mal perdrait sa session sans
    /// avoir rien fait de mal.
    renew_after_seconds: u64,
}

/// Ce qu'il rend quand il le refuse.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Held {
    module_id: String,
    machine: String,
}

/// La session en cours : le verrou local, et le bail de compte s'il a pu être pris.
pub struct DevSession {
    local: crate::watch_lock::WatchLock,
    lease: Option<Arc<LeaseHolder>>,
}

/// De quoi rendre la place depuis ailleurs — la tâche qui guette ctrl-c, notamment.
///
/// Une poignée légère, et non la session elle-même : une tâche qui retiendrait la session
/// empêcherait son `Drop` de s'exécuter au retour normal, et le verrou local survivrait à
/// chaque échec.
#[derive(Clone)]
pub struct Release {
    lock: std::path::PathBuf,
    lease: Option<Arc<LeaseHolder>>,
}

impl Release {
    /// Rend le verrou local puis le bail. Au mieux : on part de toute façon.
    pub async fn now(&self) {
        let _ = std::fs::remove_file(&self.lock);
        let Some(holder) = &self.lease else {
            return;
        };
        let _ = reqwest::Client::new()
            .delete(format!("{}/registry/v1/dev-watch", holder.base_url))
            .query(&[("sessionId", &holder.session_id)])
            .bearer_auth(&holder.token)
            .timeout(Duration::from_secs(3))
            .send()
            .await;
    }
}

struct LeaseHolder {
    base_url: String,
    session_id: String,
    token: String,
}

/// Prend la place — localement d'abord, puis auprès du registre.
///
/// Le verrou local en premier parce qu'il est immédiat et sans réseau : inutile d'aller
/// interroger le registre pour se faire refuser par sa propre machine.
pub async fn start(base_url: &str, module_id: &str, token: &str) -> Result<DevSession> {
    let local = crate::watch_lock::acquire(module_id)?;
    let session_id = uuid::Uuid::new_v4().to_string();

    let holder = Arc::new(LeaseHolder {
        base_url: base_url.trim_end_matches('/').to_string(),
        session_id,
        token: token.to_string(),
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
            "your account already has a dev --watch session on {} ({}) — stop it there, or \
             run this one without --watch",
            held.module_id,
            held.machine
        ),
        Err(unreachable) => {
            // Le registre ne répond pas. On le dit et on continue : le verrou local couvre
            // encore cette machine, et refuser de travailler pour cette raison coûterait plus
            // que la gêne qu'on évite.
            ui::warn("could not reach the registry — this session is not held account-wide");
            // La chaîne entière : le contexte seul dit ce qu'on tentait, pas ce qui a échoué.
            ui::detail(format!("{unreachable:#}"));
            ui::advice("another machine on this account could start one too");
            Ok(DevSession { local, lease: None })
        }
    }
}

impl DevSession {
    /// De quoi rendre la place depuis une autre tâche.
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

async fn hold(holder: &LeaseHolder, module_id: &str) -> Result<Kept> {
    let response = reqwest::Client::new()
        .put(format!("{}/registry/v1/dev-watch", holder.base_url))
        .bearer_auth(&holder.token)
        .json(&serde_json::json!({
            "sessionId": holder.session_id,
            "moduleId": module_id,
            "machine": machine(),
        }))
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .context("ask the registry for the watch session")?;

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
        anyhow::bail!("the registry answered {status}");
    }
    Ok(Kept::Ours(response.json().await.context("read the lease")?))
}

/// Repousse le bail tant que la session vit.
fn spawn_renewal(holder: Arc<LeaseHolder>, module_id: String, first: Lease) {
    tokio::spawn(async move {
        let every = Duration::from_secs(first.renew_after_seconds.clamp(5, 600));
        // Le bail dure plusieurs fois cet intervalle : quelques échecs d'affilée ne mettent
        // rien en danger, et se taire évite d'inquiéter pour une coupure d'une seconde.
        let mut missed = 0_u32;
        loop {
            tokio::time::sleep(every).await;
            match hold(&holder, &module_id).await {
                Ok(Kept::Ours(_)) => missed = 0,
                Ok(Kept::Theirs(held)) => {
                    // Le bail a été repris. Continuer, c'est écraser le travail de l'autre
                    // session — précisément ce que tout ceci existe pour empêcher.
                    ui::blank();
                    ui::failure(format!(
                        "this account's dev --watch session was taken over by {} ({})",
                        held.module_id, held.machine
                    ));
                    ui::detail("stopping — two sessions would undo each other's deploys");
                    std::process::exit(1);
                }
                Err(_) => {
                    missed += 1;
                    // Trois échecs : on a dépassé l'échéance, le compte est peut-être libre —
                    // ou déjà repris ailleurs.
                    if missed == 3 {
                        ui::blank();
                        ui::warn("the registry has not answered for a while");
                        ui::detail("this session may no longer be held account-wide");
                    }
                }
            }
        }
    });
}

/// Le nom de la machine, pour situer une session tenue ailleurs.
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

    /// Le nom sert à situer une session tenue ailleurs : il doit toujours dire quelque chose.
    #[test]
    fn the_machine_always_has_a_name() {
        assert!(!machine().is_empty());
    }

    /// Le serveur dicte le rythme, mais une valeur absurde ne doit pas produire une boucle
    /// serrée ni un renouvellement qui n'arrive jamais.
    #[test]
    fn a_nonsensical_interval_is_brought_back_to_reason() {
        assert_eq!(0_u64.clamp(5, 600), 5);
        assert_eq!(86_400_u64.clamp(5, 600), 600);
        assert_eq!(30_u64.clamp(5, 600), 30);
    }

    #[test]
    fn the_registry_answer_reads_as_the_cli_expects() {
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
