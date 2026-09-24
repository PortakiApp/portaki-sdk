//! Platform domain event types delivered to Wasm `#[event_handler]`s.
//!
//! Mirrors gateway event names — extend when the platform adds subscriptions
//! modules are expected to share.

use crate::ids::EventType;

/// Booking confirmed (`core.booking.confirmed`).
pub const BOOKING_CONFIRMED: EventType = EventType::new("core.booking.confirmed");

/// Module requested a transactional email via [`crate::host::email::send`].
///
/// Prefer `host::email::send` — this name is the wire event the runtime emits
/// into the gateway collector for the orchestrator generic send path.
pub const EMAIL_SEND: EventType = EventType::new("portaki.email.send");

/// Exhaustive platform event catalog known to the SDK.
pub const ALL: &[EventType] = &[BOOKING_CONFIRMED, EMAIL_SEND];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_event_wire_names() {
        assert_eq!(BOOKING_CONFIRMED.as_str(), "core.booking.confirmed");
        assert_eq!(EMAIL_SEND.as_str(), "portaki.email.send");
        assert!(!ALL.is_empty());
    }
}
