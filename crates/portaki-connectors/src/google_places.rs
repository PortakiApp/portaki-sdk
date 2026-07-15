//! Google Places connector (`connector_id = "google-places"`).
//!
//! Typed wrappers around [`portaki_sdk::host::connectors::call`] for nearby
//! search, text search, place details, and photo metadata. Structured responses
//! are provided for search operations; [`GooglePlaces::details`] and
//! [`GooglePlaces::photos`] return raw provider JSON.
//!
//! # Capabilities
//!
//! Typically `external.google-places.byok` with a property API key.
//!
//! # Example
//!
//! ```no_run
//! use portaki_connectors::google_places::{GooglePlaces, NearbySearchArgs};
//!
//! let results = GooglePlaces::nearby_search(&NearbySearchArgs {
//!     location: (43.5513, 7.0128),
//!     radius_meters: 1500,
//!     type_filter: Some("restaurant".into()),
//! })?;
//!
//! for place in &results.results {
//!     println!("{} ({}, {})", place.name, place.lat, place.lng);
//! }
//! # Ok::<(), portaki_sdk::PortakiError>(())
//! ```

use portaki_sdk::host::connectors;
use portaki_sdk::Result as SdkResult;
use serde::{Deserialize, Serialize};

/// Namespace for Google Places host connector operations.
pub struct GooglePlaces;

/// Input for [`GooglePlaces::nearby_search`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NearbySearchArgs {
    /// Search center as `(latitude, longitude)` in WGS-84 decimal degrees.
    pub location: (f64, f64),
    /// Radius around `location` in meters.
    pub radius_meters: u32,
    /// Optional Google Places `type` filter (e.g. `"restaurant"`).
    ///
    /// Omitted from the serialized JSON when `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_filter: Option<String>,
}

/// Nearby or text search result envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NearbySearchResponse {
    /// Matching places, order preserved from provider response.
    pub results: Vec<PlaceSummary>,
}

/// Single place row from a search response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaceSummary {
    /// Google `place_id` string.
    pub place_id: String,
    /// Human-readable name.
    pub name: String,
    /// WGS-84 latitude from `geometry.location.lat`.
    pub lat: f64,
    /// WGS-84 longitude from `geometry.location.lng`.
    pub lng: f64,
}

impl GooglePlaces {
    /// Runs nearby place search: `connectors::call("google-places", "nearby_search", args)`.
    ///
    /// # Errors
    ///
    /// Returns [`portaki_sdk::PortakiError`] on host or deserialization failure.
    pub fn nearby_search(args: &NearbySearchArgs) -> SdkResult<NearbySearchResponse> {
        connectors::call("google-places", "nearby_search", args)
    }

    /// Runs text search: `connectors::call("google-places", "text_search", {"query": ...})`.
    ///
    /// Response shape matches [`NearbySearchResponse`].
    pub fn text_search(query: &str) -> SdkResult<NearbySearchResponse> {
        connectors::call(
            "google-places",
            "text_search",
            &serde_json::json!({ "query": query }),
        )
    }

    /// Fetches place details by `place_id` as raw JSON.
    ///
    /// Args JSON uses camelCase key `"placeId"` expected by the gateway mapper.
    pub fn details(place_id: &str) -> SdkResult<serde_json::Value> {
        connectors::call(
            "google-places",
            "details",
            &serde_json::json!({ "placeId": place_id }),
        )
    }

    /// Fetches photo metadata for `place_id` as raw JSON.
    pub fn photos(place_id: &str) -> SdkResult<serde_json::Value> {
        connectors::call(
            "google-places",
            "photos",
            &serde_json::json!({ "placeId": place_id }),
        )
    }

    /// Validates a BYOK API key (local stub).
    ///
    /// Accepts any non-empty trimmed string; does not call Google.
    ///
    /// # Errors
    ///
    /// Returns [`super::ConnectorError::InvalidCredentials`] when `api_key` is blank.
    pub fn validate_credentials(api_key: &str) -> super::Result<()> {
        if api_key.trim().is_empty() {
            return Err(super::ConnectorError::InvalidCredentials(
                "google-places api key is empty".into(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::GooglePlaces;

    #[test]
    fn validate_credentials_rejects_blank() {
        assert!(GooglePlaces::validate_credentials("  ").is_err());
        assert!(GooglePlaces::validate_credentials("abc").is_ok());
    }
}
