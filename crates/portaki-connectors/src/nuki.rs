//! Nuki Web API connector (`connector_id = "nuki"`).
//!
//! Wraps [`portaki_sdk::host::connectors::call`] for remote lock actions.
//! Requires capability `external.nuki.byok` (workspace Nuki Web API token).
//!
//! # Example
//!
//! ```no_run
//! use portaki_connectors::nuki::{Nuki, UnlockArgs};
//!
//! let _ = Nuki::remote_unlock(&UnlockArgs {
//!     smartlock_id: "1881234".into(),
//! })?;
//! # Ok::<(), portaki_sdk::PortakiError>(())
//! ```

use portaki_sdk::host::connectors;
use portaki_sdk::Result as SdkResult;
use serde::{Deserialize, Serialize};

/// Namespace for Nuki host connector operations.
pub struct Nuki;

/// Arguments for [`Nuki::remote_unlock`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnlockArgs {
    /// Nuki smart lock id (path param `{smartlockId}`).
    pub smartlock_id: String,
}

impl Nuki {
    /// `POST /smartlock/{smartlockId}/action/unlock` via host egress (Bearer).
    pub fn remote_unlock(args: &UnlockArgs) -> SdkResult<serde_json::Value> {
        connectors::call("nuki", "remote_unlock", args)
    }
}
