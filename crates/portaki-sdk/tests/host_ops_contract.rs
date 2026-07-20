//! Contract: `contracts/host-ops.json` ↔ ExtismHostBackend dispatch wire ids.
//! Mirror lives in portaki-platform `src/test/resources/contracts/host-ops.json`.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct HostOpsContract {
    ops: Vec<String>,
}

fn contract_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../contracts/host-ops.json")
}

fn load_contract_ops() -> BTreeSet<String> {
    let raw = fs::read_to_string(contract_path()).expect("read contracts/host-ops.json");
    let contract: HostOpsContract = serde_json::from_str(&raw).expect("parse host-ops.json");
    contract.ops.into_iter().collect()
}

#[test]
fn host_ops_contract_matches_extism_host_backend() {
    let contract = load_contract_ops();
    let src = include_str!("../src/wasm/extism_host.rs");

    for op in &contract {
        let needle = format!("\"{op}\"");
        assert!(
            src.contains(&needle),
            "ExtismHostBackend missing dispatch for {op} (sync contracts/host-ops.json)"
        );
    }

    // Every dispatch_value("…") / first arg to dispatch must be in the contract.
    for line in src.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("self.dispatch_value(\"") {
            let op = rest.split('"').next().expect("dispatch_value op literal");
            assert!(
                contract.contains(op),
                "ExtismHostBackend dispatches {op:?} not listed in contracts/host-ops.json"
            );
        }
    }
}

#[test]
fn host_ops_contract_is_non_empty_unique() {
    let ops = load_contract_ops();
    assert!(!ops.is_empty());
    let raw = fs::read_to_string(contract_path()).unwrap();
    let contract: HostOpsContract = serde_json::from_str(&raw).unwrap();
    assert_eq!(
        contract.ops.len(),
        ops.len(),
        "duplicate ops in host-ops.json"
    );
}
