//! Google Places built-in connector (`google-places`).

use portaki_sdk::host::connectors;
use portaki_sdk::Result as SdkResult;
use serde::{Deserialize, Serialize};

/// Google Places connector namespace.
pub struct GooglePlaces;

/// Nearby search request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NearbySearchArgs {
    /// Center (`lat`, `lng`).
    pub location: (f64, f64),
    /// Search radius in meters.
    pub radius_meters: u32,
    /// Optional Google place type filter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_filter: Option<String>,
}

/// Nearby search response (simplified).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NearbySearchResponse {
    /// Matching places.
    pub results: Vec<PlaceSummary>,
}

/// Place summary row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaceSummary {
    /// Provider place id.
    pub place_id: String,
    /// Display name.
    pub name: String,
    /// Latitude.
    pub lat: f64,
    /// Longitude.
    pub lng: f64,
}

impl GooglePlaces {
    /// Runs `nearby_search` through the host connector runtime.
    pub fn nearby_search(args: &NearbySearchArgs) -> SdkResult<NearbySearchResponse> {
        connectors::call("google-places", "nearby_search", args)
    }

    /// Runs `text_search`.
    pub fn text_search(query: &str) -> SdkResult<NearbySearchResponse> {
        connectors::call(
            "google-places",
            "text_search",
            &serde_json::json!({ "query": query }),
        )
    }

    /// Runs `details` for a place id.
    pub fn details(place_id: &str) -> SdkResult<serde_json::Value> {
        connectors::call(
            "google-places",
            "details",
            &serde_json::json!({ "placeId": place_id }),
        )
    }

    /// Runs `photos` metadata lookup.
    pub fn photos(place_id: &str) -> SdkResult<serde_json::Value> {
        connectors::call(
            "google-places",
            "photos",
            &serde_json::json!({ "placeId": place_id }),
        )
    }

    /// Validates BYOK credentials (stub — accepts non-empty keys).
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
