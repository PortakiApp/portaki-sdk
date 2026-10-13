//! Error types surfaced by SDK host wrappers and Wasm dispatch.
//!
//! All fallible module APIs return [`Result<T>`] — never panic across the Wasm
//! boundary. Map errors to SDUI empty states or structured logs; avoid leaking
//! raw host error strings to guest-facing copy.
//!
//! ## Contract
//!
//! | Variant | Typical cause | Module response |
//! |---------|---------------|-----------------|
//! | [`PortakiError::HostNotConfigured`] | Missing mock in tests / dev | Install [`crate::host::runtime::with_host`] |
//! | [`PortakiError::CapabilityNotAvailable`] | Plan gate or missing optional cap | Fallback UX |
//! | [`PortakiError::CredentialMissing`] | BYOK not configured for a connector | Surface setup UX / degrade connector path |
//! | [`PortakiError::Storage`] | KV/repo failure | Retry or degrade read path |
//! | [`PortakiError::Connector`] | External API failure | Show connector error state |
//! | [`PortakiError::Email`] | Content over a [`crate::limits`] cap, stay ended, send cap | Fix content / stop sending |
//! | [`PortakiError::EventLimitExceeded`] | More than [`crate::limits::EVENTS_PER_INVOCATION`] emits | Batch into fewer events |
//! | [`PortakiError::Refused`] | The module refuses on purpose (`photo_required`, `host_only`) | Raise it with [`crate::refuse!`] |
//!
//! ## Declared refusals
//!
//! A module's error text never reaches the host who installed it: free text from a module is an
//! exfiltration channel (a malicious module could spell out guest data in it). What the host
//! sees is a **code**, and only a code the module declared in its manifest — a fixed, reviewed
//! vocabulary published with the version, labelled by the i18n key `errors.<code>`.
//!
//! ```ignore
//! portaki_sdk::portaki_module!(
//!     id = "lost-found",
//!     error = "photo_required",
//!     error = "host_only",
//! );
//!
//! #[command]
//! fn report(ctx: Context, params: Report) -> Result<()> {
//!     if params.photo.is_none() {
//!         return Err(portaki_sdk::refuse!("photo_required"));
//!     }
//!     Ok(())
//! }
//! ```
//!
//! On the wire the refusal reads `module_refused:photo_required`; the platform shows the code's
//! label only when the manifest's `errors` lists it, and drops anything else. Three checks keep
//! the two in step:
//!
//! - **compile time** — [`crate::refuse!`] does not compile on a code `portaki_module!` does not
//!   declare (it reads the list `portaki_module!` leaves at the crate root, so the macro must sit
//!   in `lib.rs`, as it always does);
//! - **`portaki build` / `portaki check`** — a declared code without `errors.<code>` in every
//!   bundle stops the build, and the conformance battery's i18n check says the same;
//! - **tests** — `PortakiError::Refused` built by hand skips the compile-time check:
//!   `portaki_test_utils::assert_refused` asserts both the refusal and its declaration.
//!
//! # Examples
//!
//! ```
//! use portaki_sdk::error::{PortakiError, Result};
//!
//! fn load_cache() -> Result<Vec<u8>> {
//!     match portaki_sdk::host::kv::get("weather.cache")? {
//!         Some(bytes) => Ok(bytes),
//!         None => Err(PortakiError::Host("cache miss".into())),
//!     }
//! }
//! ```

use thiserror::Error;

pub use crate::host::email::EmailError;

/// Result alias used throughout Portaki module code.
pub type Result<T> = std::result::Result<T, PortakiError>;

/// Error enum returned from host function wrappers and serialization boundaries.
#[derive(Debug, Error)]
pub enum PortakiError {
    /// No [`crate::host::runtime::HostBackend`] installed on the current thread.
    ///
    /// Production Wasm sets the backend inside the Extism shim; unit tests must
    /// wrap code in [`crate::host::runtime::with_host`].
    #[error("host functions not configured")]
    HostNotConfigured,

    /// Capability is not granted for the current property/plan.
    ///
    /// Prefer checking [`crate::Context::has_capability`] before calling gated
    /// host APIs to avoid this error in happy paths.
    #[error("capability not available: {0}")]
    CapabilityNotAvailable(String),

    /// Gateway rejected a host import with a machine-readable reason string.
    ///
    /// Treat as operational failure — log with `invocation_id` and surface a
    /// generic retry message to guests.
    #[error("host error: {0}")]
    Host(String),

    /// JSON serialization/deserialization failed crossing the Wasm FFI boundary.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Entity repository or KV operation failed.
    #[error("storage error: {0}")]
    Storage(String),

    /// Connector operation failed after egress from the gateway.
    #[error("connector error: {0}")]
    Connector(String),

    /// No credential configured for the requested provider id.
    ///
    /// Distinct from capability absence — the plan may allow BYOK but the host
    /// has not stored a key yet.
    #[error("credential not configured: {0}")]
    CredentialMissing(String),

    /// `host::email::send` refused — by the SDK before reaching the host, or by the host.
    ///
    /// See [`EmailError`] for the cases and their machine codes.
    #[error(transparent)]
    Email(#[from] EmailError),

    /// The gateway refused an event past [`crate::limits::EVENTS_PER_INVOCATION`] for this
    /// invocation (host code `event_limit_exceeded`).
    #[error(
        "event_limit_exceeded: at most {} gateway events per invocation",
        crate::limits::EVENTS_PER_INVOCATION
    )]
    EventLimitExceeded,

    /// The module refuses on purpose, with a code it declared (`portaki_module!(error = "…")`).
    ///
    /// Raise it with [`crate::refuse!`], which checks the declaration at compile time. The wire
    /// text is exactly `module_refused:<code>` — no free text: see the [module docs](self).
    #[error("module_refused:{0}")]
    Refused(&'static str),
}

/// The codes `portaki_module!(error = "…")` declares, registered for the test harness.
#[doc(hidden)]
pub struct DeclaredErrors(pub &'static [&'static str]);

inventory::collect!(DeclaredErrors);

/// Every code the modules linked into this binary declare.
pub fn declared_errors() -> impl Iterator<Item = &'static str> {
    inventory::iter::<DeclaredErrors>
        .into_iter()
        .flat_map(|declared| declared.0.iter().copied())
}

/// Whether `code` is in `declared` — `const`, for [`crate::refuse!`].
#[doc(hidden)]
pub const fn is_declared(declared: &[&str], code: &str) -> bool {
    let mut i = 0;
    while i < declared.len() {
        let (a, b) = (declared[i].as_bytes(), code.as_bytes());
        if a.len() == b.len() {
            let mut j = 0;
            while j < a.len() && a[j] == b[j] {
                j += 1;
            }
            if j == a.len() {
                return true;
            }
        }
        i += 1;
    }
    false
}

/// A declared refusal: [`PortakiError::Refused`], checked at compile time against the codes
/// `portaki_module!(error = "…")` declares.
///
/// Evaluates to the error, so it reads `return Err(refuse!("photo_required"))` or
/// `.ok_or(refuse!("host_only"))?`. See [the module docs](crate::error) for why codes are declared.
#[macro_export]
#[allow(clippy::crate_in_macro_def)] // `crate` is the caller on purpose: its declared codes.
macro_rules! refuse {
    ($code:literal) => {{
        // `crate` is the module calling the macro: `portaki_module!` left its list at its root.
        const _: () = ::core::assert!(
            $crate::error::is_declared(crate::__PORTAKI_DECLARED_ERRORS, $code),
            ::core::concat!(
                "refuse!(\"",
                $code,
                "\"): undeclared code — add error = \"",
                $code,
                "\" to portaki_module! and errors.",
                $code,
                " to every i18n bundle"
            )
        );
        $crate::error::PortakiError::Refused($code)
    }};
}

impl PortakiError {
    /// Host code for [`PortakiError::EventLimitExceeded`].
    pub const EVENT_LIMIT_EXCEEDED_CODE: &'static str = "event_limit_exceeded";

    /// Builds the error for a host dispatch failure `code` / `message`.
    ///
    /// Codes the SDK knows become typed variants, so a module can match on them instead of
    /// parsing strings; anything else stays [`PortakiError::Host`] as `"{code}: {message}"`.
    pub fn from_host_code(code: &str, message: &str) -> Self {
        if code == Self::EVENT_LIMIT_EXCEEDED_CODE {
            return Self::EventLimitExceeded;
        }
        if let Some(email) = EmailError::from_host_code(code) {
            return Self::Email(email);
        }
        Self::Host(format!("{code}: {message}"))
    }

    /// Re-types a [`PortakiError::Host`] whose text starts with a known host code.
    ///
    /// Backends other than the Extism one (test mocks, `portaki dev`) may still report host
    /// refusals as `Host("email_stay_ended: …")`; wrappers pass their result through here so
    /// the module sees the same variant whichever backend is installed.
    #[cfg_attr(not(any(feature = "email", test)), allow(dead_code))]
    pub(crate) fn typed(self) -> Self {
        match self {
            Self::Host(text) => {
                let (code, message) = text.split_once(':').unwrap_or((text.as_str(), ""));
                match Self::from_host_code(code.trim(), message.trim()) {
                    Self::Host(_) => Self::Host(text),
                    typed => typed,
                }
            }
            other => other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_host_codes_become_typed_variants() {
        assert!(matches!(
            PortakiError::from_host_code("email_stay_ended", "stay ended"),
            PortakiError::Email(EmailError::StayEnded)
        ));
        assert!(matches!(
            PortakiError::from_host_code("email_limit_exceeded", ""),
            PortakiError::Email(EmailError::LimitExceeded)
        ));
        assert!(matches!(
            PortakiError::from_host_code("event_limit_exceeded", ""),
            PortakiError::EventLimitExceeded
        ));
        let other = PortakiError::from_host_code("kv_quota", "full");
        assert_eq!(other.to_string(), "host error: kv_quota: full");
    }

    /// The platform reads the code off the front of the text `plugin_fn` hands back, which is
    /// the error's `Debug` through anyhow — for a variant without a source, its `Display`.
    #[test]
    fn a_refusal_is_its_code_on_the_wire() {
        let refusal = PortakiError::Refused("photo_required");
        assert_eq!(refusal.to_string(), "module_refused:photo_required");
        assert_eq!(
            format!("{:?}", anyhow::Error::from(refusal)),
            "module_refused:photo_required"
        );
    }

    #[test]
    fn membership_is_exact() {
        const DECLARED: &[&str] = &["photo_required", "host_only"];
        assert!(is_declared(DECLARED, "host_only"));
        assert!(!is_declared(DECLARED, "host"));
        assert!(!is_declared(DECLARED, "host_only_"));
        assert!(!is_declared(&[], "host_only"));
    }

    #[test]
    fn host_strings_carrying_a_code_are_retyped() {
        let typed = PortakiError::Host("email_limit_exceeded: 6th send".into()).typed();
        assert!(matches!(
            typed,
            PortakiError::Email(EmailError::LimitExceeded)
        ));
        let untouched = PortakiError::Host("connector_egress_failed: boom".into()).typed();
        assert_eq!(
            untouched.to_string(),
            "host error: connector_egress_failed: boom"
        );
    }
}
