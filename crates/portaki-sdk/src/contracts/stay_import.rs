//! Canonical stay row shape produced by import modules.
//!
//! Consumed by the gateway `ModuleGatewayStayImportAdapter`. Any module that
//! imports stays from an external calendar or channel manager (iCal exports,
//! Beds24, Smoobu, Lodgify, …) returns this shape so the gateway has one parser
//! instead of one per module.
//!
//! Every row carries the booking channel — see
//! [`crate::contracts::booking_channel`]. Both channel fields are **always
//! serialised**: an unidentifiable feed emits `unknown` / `none` rather than
//! omitting the keys.
//!
//! # Examples
//!
//! ```
//! use portaki_sdk::contracts::booking_channel::{BookingChannel, ChannelSignal};
//! use portaki_sdk::contracts::stay_import::StayImportRow;
//!
//! let row = StayImportRow {
//!     guest_name: "Marie Dupont".into(),
//!     guest_email: None,
//!     guest_lang: "fr".into(),
//!     check_in_at: "2026-07-20T00:00:00+00:00".into(),
//!     check_out_at: "2026-07-25T00:00:00+00:00".into(),
//!     ical_uid: "abc-123@airbnb.com".into(),
//!     booking_channel: BookingChannel::Airbnb,
//!     booking_channel_signal: ChannelSignal::IcalUidSuffix,
//! };
//!
//! let wire = serde_json::to_value(&row).unwrap();
//! assert_eq!(wire["bookingChannel"], "airbnb");
//! assert_eq!(wire["bookingChannelSignal"], "ical-uid-suffix");
//! ```

use serde::{Deserialize, Serialize};

use crate::contracts::booking_channel::{BookingChannel, ChannelSignal};

/// One importable stay (wire: camelCase JSON object).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StayImportRow {
    /// Guest display name. Import modules fall back to a generic label rather
    /// than dropping a dated reservation.
    pub guest_name: String,
    /// Guest email when the feed exposes one — most channels do not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guest_email: Option<String>,
    /// Guest language tag used for booklet copy (`fr`, `en`, …).
    #[serde(default)]
    pub guest_lang: String,
    /// Check-in instant, RFC 3339.
    pub check_in_at: String,
    /// Check-out instant, RFC 3339.
    pub check_out_at: String,
    /// Stable per-reservation key used for idempotent upserts.
    pub ical_uid: String,
    /// Who sold this stay. `unknown` when no signal identifies a seller.
    #[serde(default)]
    pub booking_channel: BookingChannel,
    /// How [`Self::booking_channel`] was established. `none` when nothing did.
    #[serde(default)]
    pub booking_channel_signal: ChannelSignal,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row() -> StayImportRow {
        StayImportRow {
            guest_name: "Ada Lovelace".into(),
            guest_email: None,
            guest_lang: "en".into(),
            check_in_at: "2026-08-01T00:00:00+00:00".into(),
            check_out_at: "2026-08-05T00:00:00+00:00".into(),
            ical_uid: "stable-1".into(),
            booking_channel: BookingChannel::Booking,
            booking_channel_signal: ChannelSignal::IcalProdid,
        }
    }

    #[test]
    fn serialises_camel_case_wire_keys() {
        let wire = serde_json::to_value(row()).expect("serialize");
        assert_eq!(wire["guestName"], "Ada Lovelace");
        assert_eq!(wire["guestLang"], "en");
        assert_eq!(wire["checkInAt"], "2026-08-01T00:00:00+00:00");
        assert_eq!(wire["checkOutAt"], "2026-08-05T00:00:00+00:00");
        assert_eq!(wire["icalUid"], "stable-1");
        assert_eq!(wire["bookingChannel"], "booking");
        assert_eq!(wire["bookingChannelSignal"], "ical-prodid");
        assert!(wire.get("guestEmail").is_none());
    }

    #[test]
    fn channel_fields_are_always_emitted_even_when_unidentified() {
        let unidentified = StayImportRow {
            booking_channel: BookingChannel::Unknown,
            booking_channel_signal: ChannelSignal::None,
            ..row()
        };
        let wire = serde_json::to_value(unidentified).expect("serialize");
        assert_eq!(wire["bookingChannel"], "unknown");
        assert_eq!(wire["bookingChannelSignal"], "none");
    }

    #[test]
    fn absent_channel_fields_deserialise_to_unknown_none() {
        let parsed: StayImportRow = serde_json::from_value(serde_json::json!({
            "guestName": "Tom Weber",
            "guestLang": "de",
            "checkInAt": "2026-09-01T00:00:00+00:00",
            "checkOutAt": "2026-09-03T00:00:00+00:00",
            "icalUid": "no-channel"
        }))
        .expect("deserialize");
        assert_eq!(parsed.booking_channel, BookingChannel::Unknown);
        assert_eq!(parsed.booking_channel_signal, ChannelSignal::None);
    }
}
