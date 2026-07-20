//! `host::time` — sandboxed clock helpers.

use chrono::{DateTime, Utc};
use std::time::Duration;

use crate::error::{PortakiError, Result};
use crate::host::runtime::backend;

/// Returns the current UTC time from the gateway clock.
pub fn now() -> Result<DateTime<Utc>> {
    let iso = backend()?.time_now_iso()?;
    DateTime::parse_from_rfc3339(&iso)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|e| PortakiError::Host(format!("time_now_parse_failed: {e}")))
}

/// Bounded sleep (gateway enforces max 5s).
pub fn sleep(duration: Duration) -> Result<()> {
    let capped = duration.min(Duration::from_secs(5));
    std::thread::sleep(capped);
    Ok(())
}
