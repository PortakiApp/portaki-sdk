//! `host::time` — sandboxed clock and formatting helpers.

use chrono::{DateTime, Utc};
use std::time::Duration;

use crate::context::fixed_now;
use crate::error::Result;
use crate::host::runtime::backend;

/// Returns the current UTC time from the gateway clock.
pub fn now() -> Result<DateTime<Utc>> {
    let _ = backend();
    Ok(fixed_now())
}

/// Returns "now" formatted for a property timezone (gateway resolves in production).
pub fn now_in_tz(tz: &str) -> Result<DateTime<Utc>> {
    let _ = tz;
    now()
}

/// Relative time phrase for display (`il y a 2h`, …).
pub fn format_relative(ts: DateTime<Utc>, locale: &str) -> Result<String> {
    let _ = locale;
    let delta = now()?.signed_duration_since(ts);
    Ok(format!("{}s ago", delta.num_seconds().abs()))
}

/// Bounded sleep (gateway enforces max 5s).
pub fn sleep(duration: Duration) -> Result<()> {
    let capped = duration.min(Duration::from_secs(5));
    std::thread::sleep(capped);
    Ok(())
}
