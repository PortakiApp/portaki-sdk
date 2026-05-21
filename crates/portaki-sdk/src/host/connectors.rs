//! `host::connectors` — typed external service calls (no raw HTTP in modules).

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::error::Result;
use crate::host::runtime::backend;

/// Invokes a built-in or custom connector operation.
pub fn call<Args, Response>(connector_id: &str, operation: &str, args: &Args) -> Result<Response>
where
    Args: Serialize,
    Response: DeserializeOwned,
{
    let args_json = serde_json::to_string(args)?;
    let response_json = backend()?.connector_call(connector_id, operation, &args_json)?;
    Ok(serde_json::from_str(&response_json)?)
}
