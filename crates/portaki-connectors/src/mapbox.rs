//! Mapbox connector (`connector_id = "mapbox"`).
//!
//! Geocoding, reverse geocoding, directions, and static map URL helpers over
//! [`portaki_sdk::host::connectors::call`]. Geocode responses are normalized to
//! [`GeocodeResponse`]; directions return raw JSON.
//!
//! # Capabilities
//!
//! Typically `external.mapbox.byok` with a Mapbox access token.
//!
//! # Example
//!
//! ```no_run
//! use portaki_connectors::mapbox::{GeocodeArgs, Mapbox, StaticMapArgs};
//!
//! let point = Mapbox::geocode(&GeocodeArgs {
//!     query: "Cannes, France".into(),
//! })?;
//!
//! let map_url = Mapbox::static_map(&StaticMapArgs {
//!     lat: point.lat,
//!     lng: point.lng,
//!     zoom: 12,
//!     width: 600,
//!     height: 400,
//! })?;
//! # Ok::<(), portaki_sdk::PortakiError>(())
//! ```

use portaki_sdk::host::connectors;
use portaki_sdk::Result as SdkResult;
use serde::{Deserialize, Serialize};

/// Namespace for Mapbox host connector operations.
pub struct Mapbox;

/// Forward-geocode input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeocodeArgs {
    /// Free-text address or place name.
    pub query: String,
}

/// First-feature geocode result.
///
/// Extracted from the top Mapbox Geocoding API feature when the host returns
/// a normalized payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeocodeResponse {
    /// Latitude of the first matched feature.
    pub lat: f64,
    /// Longitude of the first matched feature.
    pub lng: f64,
    /// Formatted place label (`place_name` or equivalent).
    pub label: String,
}

/// Static map image request parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticMapArgs {
    /// Map center latitude.
    pub lat: f64,
    /// Map center longitude.
    pub lng: f64,
    /// Zoom level (`0`–`22` per Mapbox conventions; host may clamp).
    pub zoom: u8,
    /// Viewport width in pixels.
    pub width: u32,
    /// Viewport height in pixels.
    pub height: u32,
}

impl Mapbox {
    /// Forward geocoding: `connectors::call("mapbox", "geocode", args)`.
    pub fn geocode(args: &GeocodeArgs) -> SdkResult<GeocodeResponse> {
        connectors::call("mapbox", "geocode", args)
    }

    /// Reverse geocoding: `connectors::call("mapbox", "reverse_geocode", {lat, lng})`.
    ///
    /// Returns the same [`GeocodeResponse`] shape as forward geocode.
    pub fn reverse_geocode(lat: f64, lng: f64) -> SdkResult<GeocodeResponse> {
        connectors::call(
            "mapbox",
            "reverse_geocode",
            &serde_json::json!({ "lat": lat, "lng": lng }),
        )
    }

    /// Turn-by-turn directions between two WGS-84 points.
    ///
    /// Returns unparsed Directions API JSON. Args use nested `from` / `to`
    /// objects with `lat` and `lng` fields.
    pub fn directions(from: (f64, f64), to: (f64, f64)) -> SdkResult<serde_json::Value> {
        connectors::call(
            "mapbox",
            "directions",
            &serde_json::json!({ "from": { "lat": from.0, "lng": from.1 }, "to": { "lat": to.0, "lng": to.1 } }),
        )
    }

    /// Resolves a static map image URL string for the given viewport.
    pub fn static_map(args: &StaticMapArgs) -> SdkResult<String> {
        connectors::call("mapbox", "static_map", args)
    }

    /// Validates a Mapbox access token (local stub).
    ///
    /// # Errors
    ///
    /// Returns [`super::ConnectorError::InvalidCredentials`] when `token` is blank.
    pub fn validate_credentials(token: &str) -> super::Result<()> {
        if token.trim().is_empty() {
            return Err(super::ConnectorError::InvalidCredentials(
                "mapbox token is empty".into(),
            ));
        }
        Ok(())
    }
}
