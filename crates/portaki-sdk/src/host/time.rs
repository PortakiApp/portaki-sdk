//! `host::time` — sandboxed clock helpers.

use chrono::{DateTime, Utc};

use crate::error::{PortakiError, Result};
use crate::host::runtime::backend;

/// Returns the current UTC time from the gateway clock.
pub fn now() -> Result<DateTime<Utc>> {
    let iso = backend()?.time_now_iso()?;
    DateTime::parse_from_rfc3339(&iso)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|e| PortakiError::Host(format!("time_now_parse_failed: {e}")))
}
