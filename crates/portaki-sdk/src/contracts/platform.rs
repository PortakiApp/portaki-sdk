//! Platform domain event types delivered to Wasm `#[event_handler]`s.
//!
//! Mirrors gateway event names — extend when the platform adds subscriptions
//! modules are expected to share.

use crate::ids::EventType;

/// Booking confirmed (`core.booking.confirmed`).
pub const BOOKING_CONFIRMED: EventType = EventType::new("core.booking.confirmed");

/// Exhaustive platform event catalog known to the SDK.
pub const ALL: &[EventType] = &[BOOKING_CONFIRMED];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_event_wire_names() {
        assert_eq!(BOOKING_CONFIRMED.as_str(), "core.booking.confirmed");
        assert!(!ALL.is_empty());
    }
}
