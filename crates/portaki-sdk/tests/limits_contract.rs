//! Contrat : `contracts/platform/module-limits.json` ↔ `portaki_sdk::limits`.
//!
//! Le JSON est une copie de `contracts/module-limits.json` de portaki-platform, qui fait foi ;
//! `limits.rs` le recopie en constantes. Ce test vérifie la recopie dans les deux sens :
//!
//! - chaque constante vaut ce que dit le JSON ;
//! - chaque feuille du JSON a sa constante — une limite ajoutée côté plateforme fait échouer
//!   ce test dès la synchronisation (`scripts/sync-platform-contracts.sh`), au lieu de rester
//!   ignorée du SDK.
//!
//! Aucun accès réseau ici : la copie est lue sur disque.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use portaki_sdk::limits;
use serde_json::{json, Value};

/// Le format du document que ce test sait lire. Un autre numéro veut dire que la plateforme a
/// changé la forme du contrat, pas seulement une valeur : relire la table ci-dessous.
const CONTRACT_VERSION: u64 = 1;

fn contract() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../contracts/platform/module-limits.json");
    let raw =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("lecture de {}: {e}", path.display()));
    serde_json::from_str(&raw).expect("contracts/platform/module-limits.json n'est pas du JSON")
}

/// Chemin dans le JSON → constante qui le recopie.
fn mirrored() -> Vec<(&'static str, Value)> {
    vec![
        (
            "invocation.hostCalls",
            json!(limits::HOST_CALLS_PER_INVOCATION),
        ),
        ("invocation.events", json!(limits::EVENTS_PER_INVOCATION)),
        (
            "invocation.emailSends",
            json!(limits::EMAIL_SENDS_PER_INVOCATION),
        ),
        (
            "invocation.eventPayloadBytes",
            json!(limits::EVENT_PAYLOAD_MAX_BYTES),
        ),
        (
            "invocation.connectorCalls",
            json!(limits::CONNECTOR_CALLS_PER_INVOCATION),
        ),
        (
            "connector.responseMaxBytes",
            json!(limits::CONNECTOR_RESPONSE_MAX_BYTES),
        ),
        (
            "connector.requestTimeoutMs",
            json!(limits::CONNECTOR_REQUEST_TIMEOUT_MS),
        ),
        ("connector.httpsOnly", json!(limits::CONNECTOR_HTTPS_ONLY)),
        ("kv.keyMaxBytes", json!(limits::KV_KEY_MAX_BYTES)),
        ("kv.valueMaxBytes", json!(limits::KV_VALUE_MAX_BYTES)),
        ("kv.keysPerScope", json!(limits::KV_KEYS_PER_SCOPE)),
        ("kv.bytesPerScope", json!(limits::KV_BYTES_PER_SCOPE)),
        (
            "email.subjectMaxChars",
            json!(limits::EMAIL_SUBJECT_MAX_CHARS),
        ),
        (
            "email.eyebrowMaxChars",
            json!(limits::EMAIL_EYEBROW_MAX_CHARS),
        ),
        ("email.titleMaxChars", json!(limits::EMAIL_TITLE_MAX_CHARS)),
        ("email.bodyMaxChars", json!(limits::EMAIL_BODY_MAX_CHARS)),
        (
            "email.ctaLabelMaxChars",
            json!(limits::EMAIL_CTA_LABEL_MAX_CHARS),
        ),
        (
            "email.guestDaysAfterCheckout",
            json!(limits::GUEST_EMAIL_DAYS_AFTER_CHECKOUT),
        ),
        (
            "email.guestPerStayPer24h",
            json!(limits::GUEST_STAY_EMAILS_PER_24H),
        ),
        (
            "email.guestPerStayTotal",
            json!(limits::GUEST_STAY_EMAILS_TOTAL),
        ),
        (
            "email.hostPerModulePer24h",
            json!(limits::HOST_EMAILS_PER_MODULE_PER_24H),
        ),
    ]
}

/// Chemins pointés de toutes les feuilles du document, `version` exclue.
fn leaves(value: &Value, prefix: &str, out: &mut BTreeSet<String>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                leaves(child, &path, out);
            }
        }
        _ => {
            out.insert(prefix.to_string());
        }
    }
}

fn at<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    path.split('.').try_fold(root, |node, key| node.get(key))
}

#[test]
fn contract_version_is_the_one_this_test_reads() {
    assert_eq!(
        contract().get("version").and_then(Value::as_u64),
        Some(CONTRACT_VERSION),
        "module-limits.json a changé de format — relire la table de ce test avant de monter CONTRACT_VERSION"
    );
}

#[test]
fn every_const_equals_the_platform_value() {
    let contract = contract();
    let mismatches: Vec<String> = mirrored()
        .into_iter()
        .filter_map(|(path, ours)| match at(&contract, path) {
            Some(theirs) if *theirs == ours => None,
            Some(theirs) => Some(format!("{path}: plateforme {theirs}, SDK {ours}")),
            None => Some(format!(
                "{path}: absent du contrat (retiré côté plateforme ?), SDK {ours}"
            )),
        })
        .collect();
    assert!(
        mismatches.is_empty(),
        "limits.rs ne recopie pas contracts/platform/module-limits.json :\n  {}",
        mismatches.join("\n  ")
    );
}

#[test]
fn every_platform_limit_is_mirrored() {
    let mut found = BTreeSet::new();
    leaves(&contract(), "", &mut found);
    found.remove("version");

    let mirrored: BTreeSet<String> = mirrored().iter().map(|(p, _)| p.to_string()).collect();
    let missing: Vec<&String> = found.difference(&mirrored).collect();
    assert!(
        missing.is_empty(),
        "limites de la plateforme sans constante dans limits.rs (ajouter la constante et sa ligne dans ce test) : {missing:?}"
    );
}

#[test]
fn mirror_table_has_no_duplicate_path() {
    let table = mirrored();
    let unique: BTreeSet<&str> = table.iter().map(|(p, _)| *p).collect();
    assert_eq!(
        unique.len(),
        table.len(),
        "chemin en double dans mirrored()"
    );
}
