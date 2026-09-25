//! Typed external service calls — the only supported egress path from Wasm.
//!
//! Modules declare connectors in the manifest (`connector!`, `custom_connector!`)
//! and invoke operations through [`call`]. The gateway enforces capability grants,
//! resolves credentials, and performs HTTP on behalf of the module.
//!
//! ## Limits the gateway enforces
//!
//! - At most [`crate::limits::CONNECTOR_CALLS_PER_INVOCATION`] connector calls per
//!   invocation — cache results in `host::kv` rather than calling in a loop.
//! - Responses are capped at [`crate::limits::CONNECTOR_RESPONSE_MAX_BYTES`] (1 MiB);
//!   request narrower pages or fields from the upstream API.
//! - `https` only, and targets resolving to private / loopback / link-local addresses
//!   are blocked.
//!
//! There is no per-provider rate limiting on the module's behalf: an upstream `429`
//! surfaces as a connector error like any other failure.
//!
//! ## Contract
//!
//! - `connector_id` and `operation` must match manifest entries — unknown ops fail fast.
//! - `Args` and `Response` are JSON-serializable Rust types — keep shapes stable.
//! - Errors map to [`crate::error::PortakiError::Connector`] with gateway reason strings.
//!
//! ## What modules must not assume
//!
//! - No raw `reqwest`, `ureq`, or socket access inside Wasm — blocked by policy.
//! - Response schemas are not validated in the SDK — defensively deserialize optional fields.
//! - Pool vs BYOK routing is gateway-internal — never branch on secret source in modules.
//!
//! # Examples
//!
//! ```ignore
//! use portaki_sdk::prelude::*;
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Serialize)]
//! struct WeatherArgs { lat: f64, lng: f64 }
//!
//! #[derive(Deserialize)]
//! struct WeatherResponse { temperature_c: f64 }
//!
//! /// `None` for a property not geocoded yet: the surface shows its empty state.
//! fn load_weather(ctx: &Context) -> Result<Option<WeatherResponse>> {
//!     let Some(point) = ctx.property.coordinates else {
//!         return Ok(None);
//!     };
//!     host::connectors::call(
//!         "open-weather",
//!         "current",
//!         &WeatherArgs { lat: point.lat, lng: point.lng },
//!     )
//!     .map(Some)
//! }
//! ```

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::error::Result;
use crate::host::runtime::backend;

/// Invokes `connector_id` / `operation` with typed args and response.
///
/// Serializes `args` to JSON, dispatches through [`crate::host::runtime::HostBackend::connector_call`],
/// and deserializes the response payload.
pub fn call<Args, Response>(connector_id: &str, operation: &str, args: &Args) -> Result<Response>
where
    Args: Serialize,
    Response: DeserializeOwned,
{
    let args_json = serde_json::to_string(args)?;
    let response_json = backend()?.connector_call(connector_id, operation, &args_json)?;
    Ok(serde_json::from_str(&response_json)?)
}
