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
//! **local format checks** used during module install / BYOK configuration — they
//! do not call the provider network.
//!
//! # Connector IDs and operations
//!
//! Each submodule maps 1:1 to a host connector id and its operation strings:
//!
//! | Module | `connector_id` | Operations |
//! |--------|----------------|------------|
//! | [`open_weather`] | `open-weather` | `current`, `forecast`, `historical` |
//! | [`open_agenda`] | `open-agenda` | `nearby_events` |
//! | [`google_places`] | `google-places` | `nearby_search`, `text_search`, `details`, `photos` |
//! | [`mapbox`] | `mapbox` | `geocode`, `reverse_geocode`, `directions`, `static_map` |
//! | [`osm_nominatim`] | `osm-nominatim` | `geocode`, `reverse_geocode` |
//! | [`nuki`] | `nuki` | `remote_unlock` |
//! | `tiqets` | `tiqets` | `nearby_products` — **déprécié**, voir ci-dessous |
//! | `viator` | `viator` | `search_products` — **déprécié**, voir ci-dessous |
//!
//! # Ce qui n'a pas sa place ici
//!
//! Ce crate ne porte que des connecteurs **catalogués par la plateforme** : elle en résout les
//! identifiants, en accorde les capacités et en garde les clés. Un connecteur qu'un module
//! déclare lui-même (ADR-0021) n'y a rien à faire — ses types et son analyse lui appartiennent,
//! et les mettre ici met son travail derrière une release du SDK pour rien.
//!
//! `tiqets` et `viator` sont dans ce cas, par héritage : la plateforme n'accorde plus de capacité
//! pour eux (leurs [`portaki_sdk::capability::CapabilityId`] le disent déjà), `local-guide` les a
//! rapatriés, et **plus rien ici ne les consomme**. Ils partiront en 10.0, et `portaki add
//! connector` ne les propose plus — ils ont quitté la liste des connecteurs intégrés, qu'un test
//! épingle sur le tableau ci-dessus.
//!
//! Pas de `#[deprecated]` dessus, et ce n'est pas un oubli : l'attribut sur un module fait rougir
//! les constantes que `#[test]` engendre dans ses propres tests, et aucun `allow` ne les couvre.
//! On perdrait la couverture de code qui tourne encore en 9.x pour avertir des appelants qui
//! n'existent plus. La note ci-dessus et la dépréciation des capacités disent la même chose.
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
//! # Testing
//!
//! Each connector carries canned responses for its operations ([`open_weather::MOCK_RESPONSES`],
//! …), aggregated by [`mod@mock`]. `portaki-test-utils` mounts them all with
//! `MockContextBuilder::with_builtin_connectors()`; `with_connector_response` still overrides
//! any single operation.
//!
//! They are behind the `mock` feature, off by default, so they are compiled out of a module's
//! published Wasm rather than merely unused — see [`mod@mock`] for the boundary and its proof.
//!
//! # Errors
//!
//! - Runtime connector failures surface as [`portaki_sdk::PortakiError`] from
//!   [`portaki_sdk::host::connectors::call`].
//! - [`ConnectorError`] covers local credential validation only.

#![deny(missing_docs)]

pub mod google_places;
pub mod mapbox;
#[cfg(feature = "mock")]
pub mod mock;
pub mod nuki;
pub mod open_agenda;
pub mod open_weather;
pub mod osm_nominatim;
pub mod tiqets;
pub mod viator;

pub use google_places::GooglePlaces;
pub use mapbox::Mapbox;
pub use nuki::Nuki;
pub use open_agenda::OpenAgenda;
pub use open_weather::OpenWeather;
pub use osm_nominatim::OsmNominatim;
pub use tiqets::Tiqets;
pub use viator::Viator;

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
