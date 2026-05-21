//! Mapbox built-in connector (`mapbox`).

use portaki_sdk::host::connectors;
use portaki_sdk::Result as SdkResult;
use serde::{Deserialize, Serialize};

/// Mapbox connector namespace.
pub struct Mapbox;

/// Geocode request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeocodeArgs {
    /// Free-text query.
    pub query: String,
}

/// Geocode response (simplified).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeocodeResponse {
    /// First feature lat.
    pub lat: f64,
    /// First feature lng.
    pub lng: f64,
    /// Formatted label.
    pub label: String,
}

/// Static map request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticMapArgs {
    /// Center lat.
    pub lat: f64,
    /// Center lng.
    pub lng: f64,
    /// Zoom level.
    pub zoom: u8,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

impl Mapbox {
    /// Forward geocoding.
    pub fn geocode(args: &GeocodeArgs) -> SdkResult<GeocodeResponse> {
        connectors::call("mapbox", "geocode", args)
    }

    /// Reverse geocoding.
    pub fn reverse_geocode(lat: f64, lng: f64) -> SdkResult<GeocodeResponse> {
        connectors::call(
            "mapbox",
            "reverse_geocode",
            &serde_json::json!({ "lat": lat, "lng": lng }),
        )
    }

    /// Turn-by-turn directions (simplified JSON).
    pub fn directions(from: (f64, f64), to: (f64, f64)) -> SdkResult<serde_json::Value> {
        connectors::call(
            "mapbox",
            "directions",
            &serde_json::json!({ "from": { "lat": from.0, "lng": from.1 }, "to": { "lat": to.0, "lng": to.1 } }),
        )
    }

    /// Static map image URL.
    pub fn static_map(args: &StaticMapArgs) -> SdkResult<String> {
        connectors::call("mapbox", "static_map", args)
    }

    /// Validates Mapbox token (stub).
    pub fn validate_credentials(token: &str) -> super::Result<()> {
        if token.trim().is_empty() {
            return Err(super::ConnectorError::InvalidCredentials(
                "mapbox token is empty".into(),
            ));
        }
        Ok(())
    }
}
