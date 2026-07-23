//! `access.smart_lock` peer protocol.
//!
//! Provider modules declare [`crate::capability::access::SMART_LOCK`] under
//! `capabilities.provided`. Consumers discover peers with
//! [`crate::host::module::list_by_capability`] and invoke the commands below via
//! [`crate::sdui::action::Action::command`] (target `module_id` = peer id).
//!
//! # Examples
//!
//! ```
//! use portaki_sdk::contracts::smart_lock;
//! use portaki_sdk::ids::ModuleId;
//! use portaki_sdk::sdui::action::{Action, EmptyArgs};
//!
//! let peer = ModuleId::from_static("nuki");
//! let unlock = Action::command(&peer, smart_lock::UNLOCK, EmptyArgs {});
//! let cred = Action::command(&peer, smart_lock::GET_GUEST_CREDENTIAL, EmptyArgs {});
//! assert!(matches!(unlock, Action::Command { .. }));
//! assert!(matches!(cred, Action::Command { .. }));
//! let _ = peer;
//! ```

use crate::capability::CapabilityId;
use crate::ids::OperationName;

/// Capability id providers must declare (`access.smart_lock`).
pub const CAPABILITY: CapabilityId = CapabilityId::SmartLock;

/// Remote unlock command (`unlock`).
pub const UNLOCK: OperationName = OperationName::new("unlock");

/// Fetch guest credential / code command (`getGuestCredential`).
pub const GET_GUEST_CREDENTIAL: OperationName = OperationName::new("getGuestCredential");

/// Exhaustive command catalog for providers implementing this contract.
pub const COMMANDS: &[OperationName] = &[UNLOCK, GET_GUEST_CREDENTIAL];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smart_lock_wire_names() {
        assert_eq!(CAPABILITY.as_str(), "access.smart_lock");
        assert_eq!(UNLOCK.as_str(), "unlock");
        assert_eq!(GET_GUEST_CREDENTIAL.as_str(), "getGuestCredential");
        assert_eq!(COMMANDS.len(), 2);
    }
}
