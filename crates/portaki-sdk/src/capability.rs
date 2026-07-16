//! Canonical capability identifier constants.
//!
//! Every `&str` here must match
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
//!   use [`crate::host::credentials`] rather than branching on capability strings alone.
//! - `ai::*` entries are roadmap placeholders — do not depend on them in production paths yet.
//!
//! # Examples
//!
//! ```
//! use portaki_sdk::capability::{self, core, external};
//!
//! assert!(capability::is_known(core::STORAGE));
//! assert_eq!(external::GOOGLE_PLACES_BYOK, "external.google-places.byok");
//! ```

/// Core platform capabilities and plan quotas.
pub mod core {
    /// Property count ceiling for the workspace plan.
    pub const PROPERTIES: &str = "core.properties";
    /// Booking volume quota.
    pub const BOOKINGS: &str = "core.bookings";
    /// Maximum concurrently active modules on the workspace.
    pub const MODULES_ACTIVE: &str = "core.modules.active";
    /// Guest push and in-app notification channel.
    pub const GUESTS_NOTIFICATIONS: &str = "core.guests.notifications";
    /// iCal feed import allowance.
    pub const ICAL_IMPORT: &str = "core.ical.import";
    /// iCal export feature flag.
    pub const ICAL_EXPORT: &str = "core.ical.export";
    /// Transactional email send quota.
    pub const EMAIL_TRANSACTIONAL: &str = "core.email.transactional";
    /// Custom outbound email domain.
    pub const EMAIL_CUSTOM_DOMAIN: &str = "core.email.custom_domain";
    /// Remove Portaki branding from guest surfaces.
    pub const BRANDING_REMOVE_PORTAKI: &str = "core.branding.remove_portaki";
    /// Upload a custom property logo.
    pub const BRANDING_CUSTOM_LOGO: &str = "core.branding.custom_logo";
    /// Override theme palette beyond defaults.
    pub const THEME_CUSTOM_COLORS: &str = "core.theme.custom_colors";
    /// Priority support entitlement.
    pub const SUPPORT_PRIORITY: &str = "core.support.priority";
    /// Basic analytics dashboards.
    pub const ANALYTICS_BASIC: &str = "core.analytics.basic";
    /// Advanced analytics dashboards.
    pub const ANALYTICS_ADVANCED: &str = "core.analytics.advanced";
    /// Module KV and entity repository storage (implicit on all paid tiers).
    pub const STORAGE: &str = "core.storage";
    /// Image upload and transform pipeline.
    pub const IMAGES: &str = "core.images";
}

/// External connector pool and bring-your-own-key (BYOK) capabilities.
pub mod external {
    /// Google Places — platform-managed API pool.
    pub const GOOGLE_PLACES_POOL: &str = "external.google-places.pool";
    /// Google Places — property-owned API key.
    pub const GOOGLE_PLACES_BYOK: &str = "external.google-places.byok";
    /// Mapbox — platform-managed pool.
    pub const MAPBOX_POOL: &str = "external.mapbox.pool";
    /// Mapbox — property-owned token.
    pub const MAPBOX_BYOK: &str = "external.mapbox.byok";
    /// OpenWeather — platform-managed pool.
    pub const OPEN_WEATHER_POOL: &str = "external.open-weather.pool";
    /// OpenWeather — property-owned key.
    pub const OPEN_WEATHER_BYOK: &str = "external.open-weather.byok";
    /// OpenStreetMap / Nominatim — platform-managed pool.
    pub const OSM_POOL: &str = "external.osm.pool";
}

/// AI capabilities (guest assistant is plan-mapped on Starter; others are roadmap).
pub mod ai {
    /// Inline text suggestion generation.
    pub const TEXT_SUGGESTIONS: &str = "ai.text.suggestions";
    /// Machine translation assist.
    pub const TRANSLATION: &str = "ai.translation";
    /// Generative image creation.
    pub const IMAGE_GENERATION: &str = "ai.image.generation";
    /// Guest booklet Q&A assistant (grounded on stay content).
    pub const GUEST_ASSISTANT: &str = "ai.guest.assistant";
}

/// Exhaustive list of known capability ids for compile-time validation and CLI linting.
pub const ALL: &[&str] = &[
    core::PROPERTIES,
    core::BOOKINGS,
    core::MODULES_ACTIVE,
    core::GUESTS_NOTIFICATIONS,
    core::ICAL_IMPORT,
    core::ICAL_EXPORT,
    core::EMAIL_TRANSACTIONAL,
    core::EMAIL_CUSTOM_DOMAIN,
    core::BRANDING_REMOVE_PORTAKI,
    core::BRANDING_CUSTOM_LOGO,
    core::THEME_CUSTOM_COLORS,
    core::SUPPORT_PRIORITY,
    core::ANALYTICS_BASIC,
    core::ANALYTICS_ADVANCED,
    core::STORAGE,
    core::IMAGES,
    external::GOOGLE_PLACES_POOL,
    external::GOOGLE_PLACES_BYOK,
    external::MAPBOX_POOL,
    external::MAPBOX_BYOK,
    external::OPEN_WEATHER_POOL,
    external::OPEN_WEATHER_BYOK,
    external::OSM_POOL,
    ai::TEXT_SUGGESTIONS,
    ai::TRANSLATION,
    ai::IMAGE_GENERATION,
    ai::GUEST_ASSISTANT,
];

/// Returns `true` when `id` is a registered platform capability string.
///
/// Unknown ids should be rejected at manifest merge time; this helper is for
/// defensive checks in macros and tests.
pub fn is_known(id: &str) -> bool {
    ALL.contains(&id)
}

#[cfg(test)]
mod tests {
    use super::core;
    use super::external;
    use super::ALL;

    #[test]
    fn capability_ids_match_java_enum() {
        assert!(ALL.contains(&core::STORAGE));
        assert!(ALL.contains(&external::OPEN_WEATHER_POOL));
        assert_eq!(core::PROPERTIES, "core.properties");
        assert_eq!(external::GOOGLE_PLACES_BYOK, "external.google-places.byok");
    }
}
