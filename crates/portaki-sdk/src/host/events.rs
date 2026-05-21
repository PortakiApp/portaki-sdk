//! `host::events` — emit domain events (subscriptions declared in manifest).

use serde::Serialize;

use crate::error::Result;
use crate::host::runtime::backend;

/// Emits `event_type` with a JSON payload.
pub fn emit<T: Serialize>(event_type: &str, payload: &T) -> Result<()> {
    let payload_json = serde_json::to_string(payload)?;
    backend()?.emit_event(event_type, &payload_json)
}
