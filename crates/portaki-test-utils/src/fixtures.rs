//! Default test fixtures for property/booking/guest context.

use chrono::{DateTime, Utc};
use portaki_sdk::context::{Context, GuestIdentity, PropertyContext};
use uuid::Uuid;

/// Sample property fixture.
#[derive(Debug, Clone)]
pub struct Property {
    /// Property id.
    pub id: Uuid,
    /// Display name.
    pub name: String,
    /// Latitude.
    pub lat: f64,
    /// Longitude.
    pub lng: f64,
    /// Default locale.
    pub locale: String,
    /// Timezone.
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

/// Sample booking fixture.
#[derive(Debug, Clone)]
pub struct Booking {
    /// Booking id.
    pub id: Uuid,
    /// Check-in timestamp.
    pub check_in: DateTime<Utc>,
    /// Check-out timestamp.
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

/// Guest identity fixture.
#[derive(Debug, Clone, Default)]
pub struct GuestIdentityFixture {
    /// Session id.
    pub session_id: Uuid,
    /// Display name.
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
    /// Applies this property to a [`Context`].
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
