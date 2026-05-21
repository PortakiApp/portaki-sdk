//! OSM Nominatim built-in connector (`osm-nominatim`).

use portaki_sdk::host::connectors;
use portaki_sdk::Result as SdkResult;
use serde::{Deserialize, Serialize};

/// OSM Nominatim connector namespace.
pub struct OsmNominatim;

/// Geocode response (simplified).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeocodeResponse {
    /// Latitude.
    pub lat: f64,
    /// Longitude.
    pub lng: f64,
    /// Display name.
    pub display_name: String,
}

impl OsmNominatim {
    /// Forward geocode (free tier / pool).
    pub fn geocode(query: &str) -> SdkResult<GeocodeResponse> {
        connectors::call(
            "osm-nominatim",
            "geocode",
            &serde_json::json!({ "query": query }),
        )
    }

    /// Reverse geocode.
    pub fn reverse_geocode(lat: f64, lng: f64) -> SdkResult<GeocodeResponse> {
        connectors::call(
            "osm-nominatim",
            "reverse_geocode",
            &serde_json::json!({ "lat": lat, "lng": lng }),
        )
    }

    /// Validates credentials — Nominatim pool uses no key; BYOK may use email.
    pub fn validate_credentials(token: &str) -> super::Result<()> {
        let _ = token;
        Ok(())
    }
}
