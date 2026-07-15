//! Deterministic sample data for [`portaki_sdk::context::Context`] fields.
//!
//! [`Property::default`] targets Cannes (`43.5513`, `7.0128`) with fixed UUID
//! `11111111-1111-1111-1111-111111111111`. Pass instances to
//! [`crate::MockContextBuilder::with_property`] or call [`Property::apply`] on
//! an existing context.

use chrono::{DateTime, Utc};
use portaki_sdk::context::{Context, GuestIdentity, PropertyContext};
use uuid::Uuid;

/// Sample rental property for tests.
///
/// [`Default`] yields stable id and coordinates; override fields before
/// [`Self::apply`] when scenarios need different locales or positions.
#[derive(Debug, Clone)]
pub struct Property {
    /// Property UUID written to `Context::property_id`.
    pub id: Uuid,
    /// Display name mapped to `PropertyContext::name`.
    pub name: String,
    /// WGS-84 latitude.
    pub lat: f64,
    /// WGS-84 longitude.
    pub lng: f64,
    /// BCP-47 locale (e.g. `"fr-FR"`).
    pub locale: String,
    /// IANA timezone id (e.g. `"Europe/Paris"`).
    pub timezone: String,
}

impl Default for Property {
    fn default() -> Self {
        Self {
            id: Uuid::parse_str("11111111-1111-1111-1111-111111111111").expect("uuid"),
            name: "Villa Azur".to_string(),
            lat: 43.5513,
            lng: 7.0128,
            locale: "fr-FR".to_string(),
            timezone: "Europe/Paris".to_string(),
        }
    }
}

/// Sample booking window for event-handler tests.
///
/// [`Default`] uses check-in `2026-06-01T15:00:00Z` and check-out
/// `2026-06-08T10:00:00Z`. `id` is random per instance.
#[derive(Debug, Clone)]
pub struct Booking {
    /// Booking UUID.
    pub id: Uuid,
    /// Check-in instant (UTC).
    pub check_in: DateTime<Utc>,
    /// Check-out instant (UTC).
    pub check_out: DateTime<Utc>,
}

impl Default for Booking {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            check_in: DateTime::parse_from_rfc3339("2026-06-01T15:00:00Z")
                .expect("date")
                .with_timezone(&Utc),
            check_out: DateTime::parse_from_rfc3339("2026-06-08T10:00:00Z")
                .expect("date")
                .with_timezone(&Utc),
        }
    }
}

/// Guest session fixture convertible to [`GuestIdentity`].
///
/// [`Default`] sets `session_id` to nil UUID and `display_name` to `None`.
/// [`From`] adds `locale: Some("fr-FR")`.
#[derive(Debug, Clone, Default)]
pub struct GuestIdentityFixture {
    /// Guest session UUID.
    pub session_id: Uuid,
    /// Optional display name for SDUI personalization tests.
    pub display_name: Option<String>,
}

impl From<GuestIdentityFixture> for GuestIdentity {
    fn from(value: GuestIdentityFixture) -> Self {
        GuestIdentity {
            session_id: value.session_id,
            display_name: value.display_name,
            locale: Some("fr-FR".to_string()),
        }
    }
}

impl Property {
    /// Writes property fields into `ctx` (`property_id`, `property`, `locale`, `timezone`).
    ///
    /// Sets a fixed address `"Cannes, France"` on `PropertyContext::address`.
    pub fn apply(&self, ctx: &mut Context) {
        ctx.property_id = self.id;
        ctx.property = PropertyContext {
            name: self.name.clone(),
            locale: self.locale.clone(),
            timezone: self.timezone.clone(),
            lat: self.lat,
            lng: self.lng,
            address: Some("Cannes, France".to_string()),
        };
        ctx.locale = self.locale.clone();
        ctx.timezone = self.timezone.clone();
    }
}
