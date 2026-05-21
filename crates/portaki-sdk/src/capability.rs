//! Capability ID constants — must match `app.portaki.domain.model.capability.Capability` (Java).

/// Core platform capabilities and quotas.
pub mod core {
    /// Maximum properties per workspace.
    pub const PROPERTIES: &str = "core.properties";
    /// Maximum bookings.
    pub const BOOKINGS: &str = "core.bookings";
    /// Maximum concurrently active modules.
    pub const MODULES_ACTIVE: &str = "core.modules.active";
    /// Guest push/in-app notifications.
    pub const GUESTS_NOTIFICATIONS: &str = "core.guests.notifications";
    /// iCal import quota.
    pub const ICAL_IMPORT: &str = "core.ical.import";
    /// iCal export feature.
    pub const ICAL_EXPORT: &str = "core.ical.export";
    /// Transactional email quota.
    pub const EMAIL_TRANSACTIONAL: &str = "core.email.transactional";
    /// Custom email domain.
    pub const EMAIL_CUSTOM_DOMAIN: &str = "core.email.custom_domain";
    /// Remove Portaki branding.
    pub const BRANDING_REMOVE_PORTAKI: &str = "core.branding.remove_portaki";
    /// Custom logo.
    pub const BRANDING_CUSTOM_LOGO: &str = "core.branding.custom_logo";
    /// Custom theme colors.
    pub const THEME_CUSTOM_COLORS: &str = "core.theme.custom_colors";
    /// Priority support.
    pub const SUPPORT_PRIORITY: &str = "core.support.priority";
    /// Basic analytics.
    pub const ANALYTICS_BASIC: &str = "core.analytics.basic";
    /// Advanced analytics.
    pub const ANALYTICS_ADVANCED: &str = "core.analytics.advanced";
    /// KV + repository storage (implicit).
    pub const STORAGE: &str = "core.storage";
    /// Image upload/transform.
    pub const IMAGES: &str = "core.images";
}

/// External connector pool/BYOK capabilities.
pub mod external {
    /// Google Places platform pool token.
    pub const GOOGLE_PLACES_POOL: &str = "external.google-places.pool";
    /// Google Places BYOK.
    pub const GOOGLE_PLACES_BYOK: &str = "external.google-places.byok";
    /// Mapbox platform pool.
    pub const MAPBOX_POOL: &str = "external.mapbox.pool";
    /// Mapbox BYOK.
    pub const MAPBOX_BYOK: &str = "external.mapbox.byok";
    /// OpenWeather platform pool.
    pub const OPEN_WEATHER_POOL: &str = "external.open-weather.pool";
    /// OpenWeather BYOK.
    pub const OPEN_WEATHER_BYOK: &str = "external.open-weather.byok";
    /// OpenStreetMap / Nominatim pool.
    pub const OSM_POOL: &str = "external.osm.pool";
}

/// AI roadmap capabilities (not yet mapped to plans).
pub mod ai {
    /// Text suggestions.
    pub const TEXT_SUGGESTIONS: &str = "ai.text.suggestions";
    /// Translation.
    pub const TRANSLATION: &str = "ai.translation";
    /// Image generation.
    pub const IMAGE_GENERATION: &str = "ai.image.generation";
}

/// All known capability IDs for linting and validation.
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
];

/// Returns whether `id` is a known platform capability.
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
