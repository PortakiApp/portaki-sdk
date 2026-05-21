//! Built-in Portaki connectors — typed operations and credential validation stubs.
//!
//! Modules call these types through `portaki_sdk::host::connectors::call`.

pub mod google_places;
pub mod mapbox;
pub mod open_weather;
pub mod osm_nominatim;

pub use google_places::GooglePlaces;
pub use mapbox::Mapbox;
pub use open_weather::OpenWeather;
pub use osm_nominatim::OsmNominatim;

/// Connector validation error.
#[derive(Debug, thiserror::Error)]
pub enum ConnectorError {
    /// Credentials rejected by the provider validator stub.
    #[error("invalid credentials: {0}")]
    InvalidCredentials(String),
}

/// Result alias for connector helpers.
pub type Result<T> = std::result::Result<T, ConnectorError>;
