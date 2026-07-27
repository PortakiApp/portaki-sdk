//! Canonical booking-channel vocabulary for stay import.
//!
//! Answers **who sold the stay**, which is a different question from what shape
//! an import feed has. A calendar dialect (Google Calendar, a plain `.ics`
//! export) describes transport; it never names a seller. Import modules map
//! their own feed-shape concepts onto this vocabulary and emit the wire values
//! on [`crate::contracts::stay_import::StayImportRow`].
//!
//! This module is **vocabulary only** — no capability booleans, no decision
//! table. Whether a channel is intermediated, tolerates outbound links, or
//! provides guest contact details is a platform (Java) concern; duplicating it
//! here would drift silently because nothing in this crate is code-generated
//! from the gateway.
//!
//! # Examples
//!
//! ```
//! use portaki_sdk::contracts::booking_channel::{BookingChannel, ChannelSignal};
//!
//! assert_eq!(BookingChannel::AbritelVrbo.as_str(), "abritel-vrbo");
//! assert_eq!(BookingChannel::parse("Airbnb"), Some(BookingChannel::Airbnb));
//! assert_eq!(BookingChannel::default(), BookingChannel::Unknown);
//!
//! assert_eq!(ChannelSignal::IcalUidSuffix.as_str(), "ical-uid-suffix");
//! assert_eq!(ChannelSignal::default(), ChannelSignal::None);
//! ```

use serde::{Deserialize, Serialize};

/// Closed catalog of booking channels (wire: JSON string).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BookingChannel {
    /// Airbnb.
    #[serde(rename = "airbnb")]
    Airbnb,
    /// Booking.com.
    #[serde(rename = "booking")]
    Booking,
    /// Abritel / Vrbo / HomeAway (one Expedia family).
    #[serde(rename = "abritel-vrbo")]
    AbritelVrbo,
    /// Sold by the host directly — own site, phone, repeat guest.
    #[serde(rename = "direct")]
    Direct,
    /// A booking platform Portaki has no dedicated code for yet.
    #[serde(rename = "other-platform")]
    OtherPlatform,
    /// Seller not identifiable from the available signals.
    #[serde(rename = "unknown")]
    #[default]
    Unknown,
}

impl BookingChannel {
    /// Exhaustive catalog — drives host selectors so the platform list has a
    /// single source of truth.
    pub const ALL: &'static [BookingChannel] = &[
        Self::Airbnb,
        Self::Booking,
        Self::AbritelVrbo,
        Self::Direct,
        Self::OtherPlatform,
        Self::Unknown,
    ];

    /// Stable wire string.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Airbnb => "airbnb",
            Self::Booking => "booking",
            Self::AbritelVrbo => "abritel-vrbo",
            Self::Direct => "direct",
            Self::OtherPlatform => "other-platform",
            Self::Unknown => "unknown",
        }
    }

    /// Parses a wire string, tolerating case and legacy snake_case aliases.
    ///
    /// Returns `None` for anything outside the catalog — callers decide whether
    /// an unmapped value means [`Self::Unknown`] or a hard error.
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "airbnb" => Some(Self::Airbnb),
            "booking" | "booking.com" => Some(Self::Booking),
            "abritel-vrbo" | "abritel_vrbo" | "abritel" | "vrbo" | "homeaway" => {
                Some(Self::AbritelVrbo)
            }
            "direct" => Some(Self::Direct),
            "other-platform" | "other_platform" => Some(Self::OtherPlatform),
            "unknown" => Some(Self::Unknown),
            _ => None,
        }
    }

    /// Returns `true` when this value names an actual seller.
    ///
    /// [`Self::Unknown`] is the absence of an answer, not a channel.
    pub const fn is_identified(self) -> bool {
        !matches!(self, Self::Unknown)
    }
}

/// Closed catalog of signals that can establish a [`BookingChannel`] (wire: JSON string).
///
/// Emitted alongside the channel so consumers can weigh how the value was
/// reached — a UID suffix survives proxying, a host declaration is an intention.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChannelSignal {
    /// Per-event `UID` suffix (e.g. `…@airbnb.com`). Survives re-export and mirroring.
    #[serde(rename = "ical-uid-suffix")]
    IcalUidSuffix,
    /// Calendar-scoped `PRODID`. Strong when present, cannot split a mixed feed.
    #[serde(rename = "ical-prodid")]
    IcalProdid,
    /// Derived from the feed shape the host declared.
    #[serde(rename = "feed-format-declared")]
    FeedFormatDeclared,
    /// Derived from the feed URL host at configuration time.
    #[serde(rename = "feed-url-host")]
    FeedUrlHost,
    /// The host named the selling platform explicitly.
    #[serde(rename = "host-override")]
    HostOverride,
    /// No signal — pairs with [`BookingChannel::Unknown`].
    #[serde(rename = "none")]
    #[default]
    None,
}

impl ChannelSignal {
    /// Exhaustive catalog.
    pub const ALL: &'static [ChannelSignal] = &[
        Self::IcalUidSuffix,
        Self::IcalProdid,
        Self::FeedFormatDeclared,
        Self::FeedUrlHost,
        Self::HostOverride,
        Self::None,
    ];

    /// Stable wire string.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::IcalUidSuffix => "ical-uid-suffix",
            Self::IcalProdid => "ical-prodid",
            Self::FeedFormatDeclared => "feed-format-declared",
            Self::FeedUrlHost => "feed-url-host",
            Self::HostOverride => "host-override",
            Self::None => "none",
        }
    }

    /// Parses a wire string, tolerating case and surrounding whitespace.
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "ical-uid-suffix" => Some(Self::IcalUidSuffix),
            "ical-prodid" => Some(Self::IcalProdid),
            "feed-format-declared" => Some(Self::FeedFormatDeclared),
            "feed-url-host" => Some(Self::FeedUrlHost),
            "host-override" => Some(Self::HostOverride),
            "none" => Some(Self::None),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn booking_channel_wire_names() {
        assert_eq!(BookingChannel::Airbnb.as_str(), "airbnb");
        assert_eq!(BookingChannel::Booking.as_str(), "booking");
        assert_eq!(BookingChannel::AbritelVrbo.as_str(), "abritel-vrbo");
        assert_eq!(BookingChannel::Direct.as_str(), "direct");
        assert_eq!(BookingChannel::OtherPlatform.as_str(), "other-platform");
        assert_eq!(BookingChannel::Unknown.as_str(), "unknown");
        assert_eq!(BookingChannel::ALL.len(), 6);
    }

    #[test]
    fn channel_signal_wire_names() {
        assert_eq!(ChannelSignal::IcalUidSuffix.as_str(), "ical-uid-suffix");
        assert_eq!(ChannelSignal::IcalProdid.as_str(), "ical-prodid");
        assert_eq!(
            ChannelSignal::FeedFormatDeclared.as_str(),
            "feed-format-declared"
        );
        assert_eq!(ChannelSignal::FeedUrlHost.as_str(), "feed-url-host");
        assert_eq!(ChannelSignal::HostOverride.as_str(), "host-override");
        assert_eq!(ChannelSignal::None.as_str(), "none");
        assert_eq!(ChannelSignal::ALL.len(), 6);
    }

    #[test]
    fn serde_matches_as_str_for_every_variant() {
        for channel in BookingChannel::ALL {
            assert_eq!(
                serde_json::to_value(channel).unwrap(),
                serde_json::json!(channel.as_str())
            );
            assert_eq!(BookingChannel::parse(channel.as_str()), Some(*channel));
        }
        for signal in ChannelSignal::ALL {
            assert_eq!(
                serde_json::to_value(signal).unwrap(),
                serde_json::json!(signal.as_str())
            );
            assert_eq!(ChannelSignal::parse(signal.as_str()), Some(*signal));
        }
    }

    #[test]
    fn parse_accepts_legacy_aliases_and_rejects_unknown_codes() {
        assert_eq!(
            BookingChannel::parse("abritel_vrbo"),
            Some(BookingChannel::AbritelVrbo)
        );
        assert_eq!(
            BookingChannel::parse("vrbo"),
            Some(BookingChannel::AbritelVrbo)
        );
        assert_eq!(
            BookingChannel::parse("  Booking.com "),
            Some(BookingChannel::Booking)
        );
        assert_eq!(BookingChannel::parse("google"), None);
        assert_eq!(BookingChannel::parse("generic"), None);
        assert_eq!(ChannelSignal::parse("summary"), None);
    }

    #[test]
    fn unknown_is_not_an_identified_channel() {
        assert!(BookingChannel::Airbnb.is_identified());
        assert!(BookingChannel::Direct.is_identified());
        assert!(BookingChannel::OtherPlatform.is_identified());
        assert!(!BookingChannel::Unknown.is_identified());
    }

    #[test]
    fn defaults_pair_unknown_with_no_signal() {
        assert_eq!(BookingChannel::default(), BookingChannel::Unknown);
        assert_eq!(ChannelSignal::default(), ChannelSignal::None);
    }
}
