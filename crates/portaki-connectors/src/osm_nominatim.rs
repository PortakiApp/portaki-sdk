//! OSM Nominatim connector (`connector_id = "osm-nominatim"`).
//!
//! Forward and reverse geocoding via [`portaki_sdk::host::connectors::call`].
//! Responses map to a simplified [`GeocodeResponse`] shared by both operations.
//!
//! # Capabilities
//!
//! - Platform pool — no API key; usage policy enforced by the gateway
//! - BYOK — optional contact email token for dedicated Nominatim instances
//!
//! [`OsmNominatim::validate_credentials`] always succeeds: the pool tier
//! requires no secret; BYOK email is not validated locally.
//!
//! # Example
//!
//! ```no_run
//! use portaki_connectors::osm_nominatim::OsmNominatim;
//!
//! let place = OsmNominatim::geocode("Cannes, France")?;
//! let address = OsmNominatim::reverse_geocode(place.lat, place.lng)?;
//! # Ok::<(), portaki_sdk::PortakiError>(())
//! ```

use portaki_sdk::host::connectors;
use portaki_sdk::Result as SdkResult;
use serde::{Deserialize, Serialize};

/// Namespace for OSM Nominatim host connector operations.
pub struct OsmNominatim;

/// Geocode or reverse-geocode result.
///
/// Both [`OsmNominatim::geocode`] and [`OsmNominatim::reverse_geocode`] deserialize
/// into this struct.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeocodeResponse {
    /// WGS-84 latitude (`lat` field from Nominatim).
    pub lat: f64,
    /// WGS-84 longitude (`lon` mapped to `lng`).
    pub lng: f64,
    /// Full `display_name` string from Nominatim.
    pub display_name: String,
}

impl OsmNominatim {
    /// Forward geocode: `connectors::call("osm-nominatim", "geocode", {"query": ...})`.
    pub fn geocode(query: &str) -> SdkResult<GeocodeResponse> {
        connectors::call(
            "osm-nominatim",
            "geocode",
            &serde_json::json!({ "query": query }),
        )
    }

    /// Reverse geocode: `connectors::call("osm-nominatim", "reverse_geocode", {lat, lng})`.
    pub fn reverse_geocode(lat: f64, lng: f64) -> SdkResult<GeocodeResponse> {
        connectors::call(
            "osm-nominatim",
            "reverse_geocode",
            &serde_json::json!({ "lat": lat, "lng": lng }),
        )
    }

    /// Credential check for install-time validation (no-op stub).
    ///
    /// Pool usage needs no token. BYOK email strings are accepted without
    /// format validation. Always returns `Ok(())`.
    pub fn validate_credentials(token: &str) -> super::Result<()> {
        let _ = token;
        Ok(())
    }
}
