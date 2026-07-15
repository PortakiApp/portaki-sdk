//! Typed clients for Portaki built-in external connectors.
//!
//! # Role in the SDK stack
//!
//! `portaki-connectors` sits beside [`portaki_sdk`]: modules depend on both.
//! [`portaki_sdk::host::connectors::call`] is the low-level host dispatch
//! (serialize args → JSON egress → deserialize response). This crate wraps that
//! dispatch with provider-specific types, operation names, and response parsing.
//!
//! Modules never perform raw HTTP. They either:
//!
//! 1. Call types here (e.g. [`OpenWeather::current`]) from domain code, or
//! 2. Declare connector metadata in their manifest via `#[portaki_sdk::custom_connector]`
//!    while still using these types at runtime.
//!
//! Credential validation helpers ([`OpenWeather::validate_credentials`], etc.) are
//! local stubs used during module install / BYOK configuration — they do not hit
//! the network.
//!
//! # Connector IDs and operations
//!
//! Each submodule maps 1:1 to a host connector id and its operation strings:
//!
//! | Module | `connector_id` | Operations |
//! |--------|----------------|------------|
//! | [`open_weather`] | `open-weather` | `current`, `forecast`, `historical` |
//! | [`google_places`] | `google-places` | `nearby_search`, `text_search`, `details`, `photos` |
//! | [`mapbox`] | `mapbox` | `geocode`, `reverse_geocode`, `directions`, `static_map` |
//! | [`osm_nominatim`] | `osm-nominatim` | `geocode`, `reverse_geocode` |
//!
//! The gateway resolves credentials (platform pool or BYOK) from the invocation
//! [`portaki_sdk::context::Context`] capabilities before executing egress.
//!
//! # Usage
//!
//! ```no_run
//! use portaki_connectors::open_weather::{CurrentArgs, OpenWeather};
//! use portaki_sdk::host::{self, HostBackend};
//! use portaki_sdk::context::Context;
//! use portaki_sdk::PortakiError;
//! use std::sync::Arc;
//!
//! let backend: Arc<dyn HostBackend> = todo!("install backend");
//! let ctx = Context::default();
//!
//! let temp_c = host::with_host(backend, ctx, || -> Result<f64, PortakiError> {
//!     let weather = OpenWeather::current(&CurrentArgs { lat: 43.55, lng: 7.01 })?;
//!     Ok(weather.temp_c)
//! })?;
//! # Ok::<(), PortakiError>(())
//! ```
//!
//! In unit tests, pair this crate with `portaki-test-utils` to stub connector
//! JSON responses via `MockContextBuilder::with_connector_response`.
//!
//! # Errors
//!
//! - Runtime connector failures surface as [`portaki_sdk::PortakiError`] from
//!   [`portaki_sdk::host::connectors::call`].
//! - [`ConnectorError`] covers local credential validation only.

#![deny(missing_docs)]

pub mod google_places;
pub mod mapbox;
pub mod open_weather;
pub mod osm_nominatim;

pub use google_places::GooglePlaces;
pub use mapbox::Mapbox;
pub use open_weather::OpenWeather;
pub use osm_nominatim::OsmNominatim;

/// Local validation failure for connector credentials (install-time / BYOK checks).
///
/// Distinct from [`portaki_sdk::PortakiError`]: returned only by
/// `validate_credentials` helpers in this crate. Does not indicate an egress
/// or host dispatch failure.
#[derive(Debug, thiserror::Error)]
pub enum ConnectorError {
    /// The supplied API key or token failed the crate-local non-empty check.
    ///
    /// Message is provider-specific (e.g. `"open-weather api key is empty"`).
    #[error("invalid credentials: {0}")]
    InvalidCredentials(String),
}

/// Result alias for [`ConnectorError`].
///
/// Used by `validate_credentials` methods. Connector runtime calls return
/// [`portaki_sdk::Result`] instead.
pub type Result<T> = std::result::Result<T, ConnectorError>;
