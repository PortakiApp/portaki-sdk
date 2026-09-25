//! In-process test harness for Portaki module unit tests.
//!
//! # Role in the SDK stack
//!
//! `portaki-test-utils` complements [`portaki_sdk`] for **dev-dependencies** in
//! module crates. Production Wasm modules call host functions through
//! [`portaki_sdk::host`] (KV, i18n, connectors, repo, …). This crate supplies
//! [`MockHostFunctions`], a [`portaki_sdk::host::HostBackend`] that runs entirely
//! in memory on the test thread.
//!
//! Typical test flow:
//!
//! 1. Build a [`portaki_sdk::Context`] with [`MockContextBuilder::guest`] or
//!    [`MockContextBuilder::host`].
//! 2. Seed translations, KV bytes, and connector JSON stubs.
//! 3. Call [`MockContextBuilder::run`] to install the mock backend via
//!    [`portaki_sdk::host::with_host`] and execute module code.
//! 4. Assert rendered SDUI with [`SurfaceAssertions`]. It walks every primitive of the
//!    contract, so a module needs no `contains_component_type` helper of its own:
//!
//! ```
//! use portaki_sdk::sdui::primitives::{Card, EmptyState, Stack};
//! use portaki_sdk::sdui::surface::Surface;
//! use portaki_test_utils::SurfaceAssertions;
//!
//! let surface = Surface::new(Stack::new().child(Card::new()));
//! let tree = SurfaceAssertions::new(&surface);
//!
//! // Was: assert!(contains_component_type(&surface, "Card"));
//! assert!(tree.contains_type("Card"));
//! assert!(!tree.contains_primitive::<EmptyState>());
//! ```
//!
//! # Relationship to `portaki-connectors`
//!
//! When module code calls `portaki_connectors::OpenWeather::current`, the mock
//! host returns JSON registered with
//! [`MockContextBuilder::with_connector_response`]. No network or gateway
//! process is required.
//!
//! # Example
//!
//! ```
//! use portaki_sdk::capability::{core, external};
//! use portaki_test_utils::{MockContext, Property};
//!
//! MockContext::guest()
//!     .with_property(Property::default())
//!     .with_capabilities(&[core::STORAGE, external::OPEN_WEATHER_POOL])
//!     .with_connector_response(
//!         "open-weather",
//!         "current",
//!         r#"{"main":{"temp":21.5,"humidity":55},"weather":[{"main":"Clear"}]}"#,
//!     )
//!     .run(|_ctx| {
//!         // Module code calling portaki_connectors::OpenWeather::current reads the stub.
//!     });
//! ```
//!
//! # Layout
//!
//! - [`MockContextBuilder`] / [`MockHostFunctions`] — mock host installation
//! - [`Property`], [`Booking`], [`GuestIdentityFixture`] — default fixtures
//! - [`SurfaceAssertions`], [`PrimitiveTag`] — SDUI tree queries over every primitive
//! - [`conformance!`] / [`mod@conformance`] — the battery every module runs: manifest, listing,
//!   surfaces, operations, i18n, emails, contracts

#![deny(missing_docs)]

mod assertions;
pub mod conformance;
mod fixtures;
mod mock_host;

#[doc(hidden)]
pub mod __private {
    pub use portaki_sdk;
}

pub use assertions::{PrimitiveTag, SurfaceAssertions};
pub use fixtures::{Booking, GuestIdentityFixture, Property};
pub use mock_host::{ConnectorCall, LogLine, MockContext, MockContextBuilder, MockHostFunctions};
