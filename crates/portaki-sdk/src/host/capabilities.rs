//! Capability probes — server-side grant hints.
//!
//! Prefer [`crate::Context::has_capability`] in surface renderers — the context
//! snapshot is already on the stack. Use this module when you need a live probe
//! after external state changes or when context is unavailable.
//!
//! ## Contract
//!
//! - [`has`] returns the effective grant for the **current property** at call time.
//! - Metering / quota values are enforced gateway-side; they are not exposed as a
//!   host probe in this SDK version.
//!
//! ## What modules must not assume
//!
//! - A `true` from [`has`] does not bypass connector credential requirements.
//! - Quota exhaustion may still fail host calls with [`crate::error::PortakiError::Host`]
//!   even when the capability id is granted.
//!
//! # Examples
//!
//! ```
//! use portaki_sdk::capability::core;
//! use portaki_sdk::context::Context;
//!
//! // Prefer context checks in renderers:
//! let ctx = Context::with_capabilities(&[core::IMAGES]);
//! assert!(ctx.has_capability(core::IMAGES));
//! ```

use crate::capability::CapabilityId;
use crate::error::Result;
use crate::host::runtime::{backend, context_or_load};

/// Returns whether the current property has capability `id` granted.
///
/// Uses the installed [`crate::host::runtime::HostBackend`] when present;
/// otherwise falls back to [`crate::Context::has_capability`] from thread-local context.
pub fn has(id: CapabilityId) -> Result<bool> {
    if let Ok(host) = backend() {
        return host.has_capability(id.as_str());
    }
    Ok(context_or_load()?.has_capability(id))
}
