//! `host::events` — emit domain events (subscriptions declared in manifest).
//!
//! Use [`crate::ids::EventType`] / [`crate::define_event_types!`] for module
//! emits, and [`crate::contracts::platform`] for platform event names used in
//! `#[event_handler]`. Bare strings are not accepted.

use serde::Serialize;

use crate::error::{PortakiError, Result};
use crate::host::runtime::backend;
use crate::ids::EventType;

/// Emits `event_type` with a JSON payload.
///
/// `event_type` must be an [`EventType`] from a module catalog or SDK contract.
///
/// The gateway accepts at most [`crate::limits::EVENTS_PER_INVOCATION`] events per
/// invocation; past that it answers [`crate::error::PortakiError::EventLimitExceeded`].
pub fn emit<T: Serialize>(event_type: EventType, payload: &T) -> Result<()> {
    let payload_json = serde_json::to_string(payload)?;
    backend()?
        .emit_event(event_type.as_str(), &payload_json)
        .map_err(PortakiError::typed)
}
