//! Client-side shell events (`Action::Emit`) owned by host/guest shells.
//!
//! These never round-trip into Wasm; the shell handles them locally.

use crate::ids::EventType;

/// Host dashboard surface form input bus (`host.surface.input`).
///
/// Used by host settings surfaces to push field patches into the shell
/// without a Wasm command round-trip.
pub const SURFACE_INPUT: EventType = EventType::new("host.surface.input");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_event_wire_names() {
        assert_eq!(SURFACE_INPUT.as_str(), "host.surface.input");
    }
}
