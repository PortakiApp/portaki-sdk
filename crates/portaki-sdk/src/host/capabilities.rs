//! `host::capabilities` — server-side capability checks (hint only in modules).

use crate::context::Quota;
use crate::error::Result;
use crate::host::runtime::{backend, context_or_load};

/// Returns whether the current property has `id` granted.
pub fn has(id: &str) -> Result<bool> {
    if let Ok(host) = backend() {
        return host.has_capability(id);
    }
    Ok(context_or_load()?.has_capability(id))
}

/// Returns quota usage for a quota-style capability, if applicable.
pub fn quota(id: &str) -> Result<Option<Quota>> {
    let _ = id;
    // Detailed quota plumbing is gateway-side; modules receive hints via context in v1.
    Ok(None)
}
