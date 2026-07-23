//! Canonical capability identifier catalog.
//!
//! Every variant here must match
//! `app.portaki.domain.model.capability.Capability` on the Java gateway.
//! Use these in `#[capability(required)]` / `#[capability(optional)]` attributes
//! and in runtime checks — never invent ad-hoc capability strings.
//!
//! ## Contract
//!
//! - **Required** capabilities block module installation when absent on the workspace plan.
//! - **Optional** capabilities may be missing; surfaces must ship fallback UX (manifest
//!   `fallback_key` + `ctx.has_capability` guards).
//! - **Quota-style** capabilities (`core.email.transactional`, …) enforce limits server-side;
//!   modules see usage hints via [`crate::context::Quota`] when exposed.
//!
//! ## What modules must not assume
//!
//! - Granting a capability does not imply unlimited usage — check quotas where relevant.
//! - `external.*.pool` vs `external.*.byok` are mutually exclusive credential paths;
//!   branch on these capability ids (and connector errors), not invent ad-hoc probes.
//! - `ai::*` entries are roadmap placeholders — do not depend on them in production paths yet.
//!
//! # Examples
//!
//! ```
//! use portaki_sdk::capability::{self, core, external, CapabilityId};
//!
//! assert!(CapabilityId::is_known(core::STORAGE.as_str()));
//! assert_eq!(external::GOOGLE_PLACES_BYOK.as_str(), "external.google-places.byok");
//! assert_eq!(core::STORAGE, CapabilityId::Storage);
//! ```

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Closed catalog of platform capability ids (wire format: JSON string).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CapabilityId {
    /// Property count ceiling for the workspace plan.
    #[serde(rename = "core.properties")]
    Properties,
    /// Booking volume quota.
    #[serde(rename = "core.bookings")]
    Bookings,
    /// Maximum concurrently active modules on the workspace.
    #[serde(rename = "core.modules.active")]
    ModulesActive,
    /// Guest push and in-app notification channel.
    #[serde(rename = "core.guests.notifications")]
    GuestsNotifications,
    /// iCal feed import allowance.
    #[serde(rename = "core.ical.import")]
    IcalImport,
    /// iCal export feature flag.
    #[serde(rename = "core.ical.export")]
    IcalExport,
    /// Transactional email send quota.
    #[serde(rename = "core.email.transactional")]
    EmailTransactional,
    /// Custom outbound email domain.
    #[serde(rename = "core.email.custom_domain")]
    EmailCustomDomain,
    /// Remove Portaki branding from guest surfaces.
    #[serde(rename = "core.branding.remove_portaki")]
    BrandingRemovePortaki,
    /// Upload a custom property logo.
    #[serde(rename = "core.branding.custom_logo")]
    BrandingCustomLogo,
    /// Override theme palette beyond defaults.
    #[serde(rename = "core.theme.custom_colors")]
    ThemeCustomColors,
    /// Priority support entitlement.
    #[serde(rename = "core.support.priority")]
    SupportPriority,
    /// Basic analytics dashboards.
    #[serde(rename = "core.analytics.basic")]
    AnalyticsBasic,
    /// Advanced analytics dashboards.
    #[serde(rename = "core.analytics.advanced")]
    AnalyticsAdvanced,
    /// Module KV and entity repository storage (implicit on all paid tiers).
    #[serde(rename = "core.storage")]
    Storage,
    /// Image upload and transform pipeline.
    #[serde(rename = "core.images")]
    Images,
    /// Google Places — platform-managed API pool.
    #[serde(rename = "external.google-places.pool")]
    GooglePlacesPool,
    /// Google Places — property-owned API key.
    #[serde(rename = "external.google-places.byok")]
    GooglePlacesByok,
    /// Mapbox — platform-managed pool.
    #[serde(rename = "external.mapbox.pool")]
    MapboxPool,
    /// Mapbox — property-owned token.
    #[serde(rename = "external.mapbox.byok")]
    MapboxByok,
    /// OpenWeather — platform-managed pool.
    #[serde(rename = "external.open-weather.pool")]
    OpenWeatherPool,
    /// OpenWeather — property-owned key.
    #[serde(rename = "external.open-weather.byok")]
    OpenWeatherByok,
    /// OpenStreetMap / Nominatim — platform-managed pool.
    #[serde(rename = "external.osm.pool")]
    OsmPool,
    /// Inline text suggestion generation.
    #[serde(rename = "ai.text.suggestions")]
    TextSuggestions,
    /// Machine translation assist.
    #[serde(rename = "ai.translation")]
    Translation,
    /// Generative image creation.
    #[serde(rename = "ai.image.generation")]
    ImageGeneration,
    /// Guest booklet Q&A assistant (grounded on stay content).
    #[serde(rename = "ai.guest.assistant")]
    GuestAssistant,
    /// Smart-lock provider contract (`getGuestCredential`, `unlock`).
    #[serde(rename = "access.smart_lock")]
    SmartLock,
}

impl CapabilityId {
    /// Stable wire id matching the Java `Capability` enum.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Properties => "core.properties",
            Self::Bookings => "core.bookings",
            Self::ModulesActive => "core.modules.active",
            Self::GuestsNotifications => "core.guests.notifications",
            Self::IcalImport => "core.ical.import",
            Self::IcalExport => "core.ical.export",
            Self::EmailTransactional => "core.email.transactional",
            Self::EmailCustomDomain => "core.email.custom_domain",
            Self::BrandingRemovePortaki => "core.branding.remove_portaki",
            Self::BrandingCustomLogo => "core.branding.custom_logo",
            Self::ThemeCustomColors => "core.theme.custom_colors",
            Self::SupportPriority => "core.support.priority",
            Self::AnalyticsBasic => "core.analytics.basic",
            Self::AnalyticsAdvanced => "core.analytics.advanced",
            Self::Storage => "core.storage",
            Self::Images => "core.images",
            Self::GooglePlacesPool => "external.google-places.pool",
            Self::GooglePlacesByok => "external.google-places.byok",
            Self::MapboxPool => "external.mapbox.pool",
            Self::MapboxByok => "external.mapbox.byok",
            Self::OpenWeatherPool => "external.open-weather.pool",
            Self::OpenWeatherByok => "external.open-weather.byok",
            Self::OsmPool => "external.osm.pool",
            Self::TextSuggestions => "ai.text.suggestions",
            Self::Translation => "ai.translation",
            Self::ImageGeneration => "ai.image.generation",
            Self::GuestAssistant => "ai.guest.assistant",
            Self::SmartLock => "access.smart_lock",
        }
    }

    /// Exhaustive catalog for compile-time validation and CLI linting.
    pub const ALL: &'static [CapabilityId] = &[
        Self::Properties,
        Self::Bookings,
        Self::ModulesActive,
        Self::GuestsNotifications,
        Self::IcalImport,
        Self::IcalExport,
        Self::EmailTransactional,
        Self::EmailCustomDomain,
        Self::BrandingRemovePortaki,
        Self::BrandingCustomLogo,
        Self::ThemeCustomColors,
        Self::SupportPriority,
        Self::AnalyticsBasic,
        Self::AnalyticsAdvanced,
        Self::Storage,
        Self::Images,
        Self::GooglePlacesPool,
        Self::GooglePlacesByok,
        Self::MapboxPool,
        Self::MapboxByok,
        Self::OpenWeatherPool,
        Self::OpenWeatherByok,
        Self::OsmPool,
        Self::TextSuggestions,
        Self::Translation,
        Self::ImageGeneration,
        Self::GuestAssistant,
        Self::SmartLock,
    ];

    /// Returns `true` when `id` is a registered platform capability string.
    pub fn is_known(id: &str) -> bool {
        Self::from_str(id).is_ok()
    }
}

impl AsRef<str> for CapabilityId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for CapabilityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for CapabilityId {
    type Err = ParseCapabilityIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "core.properties" => Ok(Self::Properties),
            "core.bookings" => Ok(Self::Bookings),
            "core.modules.active" => Ok(Self::ModulesActive),
            "core.guests.notifications" => Ok(Self::GuestsNotifications),
            "core.ical.import" => Ok(Self::IcalImport),
            "core.ical.export" => Ok(Self::IcalExport),
            "core.email.transactional" => Ok(Self::EmailTransactional),
            "core.email.custom_domain" => Ok(Self::EmailCustomDomain),
            "core.branding.remove_portaki" => Ok(Self::BrandingRemovePortaki),
            "core.branding.custom_logo" => Ok(Self::BrandingCustomLogo),
            "core.theme.custom_colors" => Ok(Self::ThemeCustomColors),
            "core.support.priority" => Ok(Self::SupportPriority),
            "core.analytics.basic" => Ok(Self::AnalyticsBasic),
            "core.analytics.advanced" => Ok(Self::AnalyticsAdvanced),
            "core.storage" => Ok(Self::Storage),
            "core.images" => Ok(Self::Images),
            "external.google-places.pool" => Ok(Self::GooglePlacesPool),
            "external.google-places.byok" => Ok(Self::GooglePlacesByok),
            "external.mapbox.pool" => Ok(Self::MapboxPool),
            "external.mapbox.byok" => Ok(Self::MapboxByok),
            "external.open-weather.pool" => Ok(Self::OpenWeatherPool),
            "external.open-weather.byok" => Ok(Self::OpenWeatherByok),
            "external.osm.pool" => Ok(Self::OsmPool),
            "ai.text.suggestions" => Ok(Self::TextSuggestions),
            "ai.translation" => Ok(Self::Translation),
            "ai.image.generation" => Ok(Self::ImageGeneration),
            "ai.guest.assistant" => Ok(Self::GuestAssistant),
            "access.smart_lock" => Ok(Self::SmartLock),
            other => Err(ParseCapabilityIdError {
                id: other.to_string(),
            }),
        }
    }
}

impl TryFrom<&str> for CapabilityId {
    type Error = ParseCapabilityIdError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::from_str(value)
    }
}

/// Error returned when parsing an unknown capability id string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseCapabilityIdError {
    /// The unrecognized capability id.
    pub id: String,
}

impl fmt::Display for ParseCapabilityIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown capability id: {}", self.id)
    }
}

impl std::error::Error for ParseCapabilityIdError {}

/// Core platform capabilities and plan quotas.
pub mod core {
    use super::CapabilityId;

    /// Property count ceiling for the workspace plan.
    pub const PROPERTIES: CapabilityId = CapabilityId::Properties;
    /// Booking volume quota.
    pub const BOOKINGS: CapabilityId = CapabilityId::Bookings;
    /// Maximum concurrently active modules on the workspace.
    pub const MODULES_ACTIVE: CapabilityId = CapabilityId::ModulesActive;
    /// Guest push and in-app notification channel.
    pub const GUESTS_NOTIFICATIONS: CapabilityId = CapabilityId::GuestsNotifications;
    /// iCal feed import allowance.
    pub const ICAL_IMPORT: CapabilityId = CapabilityId::IcalImport;
    /// iCal export feature flag.
    pub const ICAL_EXPORT: CapabilityId = CapabilityId::IcalExport;
    /// Transactional email send quota.
    pub const EMAIL_TRANSACTIONAL: CapabilityId = CapabilityId::EmailTransactional;
    /// Custom outbound email domain.
    pub const EMAIL_CUSTOM_DOMAIN: CapabilityId = CapabilityId::EmailCustomDomain;
    /// Remove Portaki branding from guest surfaces.
    pub const BRANDING_REMOVE_PORTAKI: CapabilityId = CapabilityId::BrandingRemovePortaki;
    /// Upload a custom property logo.
    pub const BRANDING_CUSTOM_LOGO: CapabilityId = CapabilityId::BrandingCustomLogo;
    /// Override theme palette beyond defaults.
    pub const THEME_CUSTOM_COLORS: CapabilityId = CapabilityId::ThemeCustomColors;
    /// Priority support entitlement.
    pub const SUPPORT_PRIORITY: CapabilityId = CapabilityId::SupportPriority;
    /// Basic analytics dashboards.
    pub const ANALYTICS_BASIC: CapabilityId = CapabilityId::AnalyticsBasic;
    /// Advanced analytics dashboards.
    pub const ANALYTICS_ADVANCED: CapabilityId = CapabilityId::AnalyticsAdvanced;
    /// Module KV and entity repository storage (implicit on all paid tiers).
    pub const STORAGE: CapabilityId = CapabilityId::Storage;
    /// Image upload and transform pipeline.
    pub const IMAGES: CapabilityId = CapabilityId::Images;
}

/// External connector pool and bring-your-own-key (BYOK) capabilities.
pub mod external {
    use super::CapabilityId;

    /// Google Places — platform-managed API pool.
    pub const GOOGLE_PLACES_POOL: CapabilityId = CapabilityId::GooglePlacesPool;
    /// Google Places — property-owned API key.
    pub const GOOGLE_PLACES_BYOK: CapabilityId = CapabilityId::GooglePlacesByok;
    /// Mapbox — platform-managed pool.
    pub const MAPBOX_POOL: CapabilityId = CapabilityId::MapboxPool;
    /// Mapbox — property-owned token.
    pub const MAPBOX_BYOK: CapabilityId = CapabilityId::MapboxByok;
    /// OpenWeather — platform-managed pool.
    pub const OPEN_WEATHER_POOL: CapabilityId = CapabilityId::OpenWeatherPool;
    /// OpenWeather — property-owned key.
    pub const OPEN_WEATHER_BYOK: CapabilityId = CapabilityId::OpenWeatherByok;
    /// OpenStreetMap / Nominatim — platform-managed pool.
    pub const OSM_POOL: CapabilityId = CapabilityId::OsmPool;
}

/// AI capabilities (guest assistant is plan-mapped on Starter; others are roadmap).
pub mod ai {
    use super::CapabilityId;

    /// Inline text suggestion generation.
    pub const TEXT_SUGGESTIONS: CapabilityId = CapabilityId::TextSuggestions;
    /// Machine translation assist.
    pub const TRANSLATION: CapabilityId = CapabilityId::Translation;
    /// Generative image creation.
    pub const IMAGE_GENERATION: CapabilityId = CapabilityId::ImageGeneration;
    /// Guest booklet Q&A assistant (grounded on stay content).
    pub const GUEST_ASSISTANT: CapabilityId = CapabilityId::GuestAssistant;
}

/// Guest access composition capabilities (smart locks, future access providers).
pub mod access {
    use super::CapabilityId;

    /// Smart-lock provider contract (`getGuestCredential`, `unlock`).
    ///
    /// Declared by lock provider modules. Consumer modules bind a provider by
    /// module id when an installed module satisfies this capability.
    ///
    /// Peer command names: [`crate::contracts::smart_lock`].
    pub const SMART_LOCK: CapabilityId = CapabilityId::SmartLock;
}

/// Exhaustive list of known capability ids for compile-time validation and CLI linting.
pub const ALL: &[CapabilityId] = CapabilityId::ALL;

/// Returns `true` when `id` is a registered platform capability string.
///
/// Unknown ids should be rejected at manifest merge time; this helper is for
/// defensive checks in macros and tests.
pub fn is_known(id: &str) -> bool {
    CapabilityId::is_known(id)
}

#[cfg(test)]
mod tests {
    use super::access;
    use super::core;
    use super::external;
    use super::CapabilityId;
    use super::ALL;
    use std::str::FromStr;

    #[test]
    fn capability_ids_match_java_enum() {
        assert!(ALL.contains(&core::STORAGE));
        assert!(ALL.contains(&external::OPEN_WEATHER_POOL));
        assert!(ALL.contains(&access::SMART_LOCK));
        assert_eq!(core::PROPERTIES.as_str(), "core.properties");
        assert_eq!(
            external::GOOGLE_PLACES_BYOK.as_str(),
            "external.google-places.byok"
        );
        assert_eq!(access::SMART_LOCK.as_str(), "access.smart_lock");
        assert_eq!(
            CapabilityId::from_str("core.storage").unwrap(),
            CapabilityId::Storage
        );
        assert!(serde_json::to_value(CapabilityId::Storage).unwrap() == "core.storage");
    }
}
