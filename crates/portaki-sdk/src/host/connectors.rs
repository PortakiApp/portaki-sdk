//! Typed external service calls — the only supported egress path from Wasm.
//!
//! Modules declare connectors in the manifest (`connector!`, `custom_connector!`)
//! and invoke operations through [`call`]. The gateway enforces capability grants,
//! resolves credentials, applies rate limits, and performs HTTP on behalf of the module.
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
//! fn load_weather(ctx: &Context) -> Result<WeatherResponse> {
//!     host::connectors::call(
//!         "open-weather",
//!         "current",
//!         &WeatherArgs { lat: ctx.property.lat, lng: ctx.property.lng },
//!     )
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
