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
//! 4. Assert rendered SDUI with [`SurfaceAssertions`].
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
//! use portaki_test_utils::{MockContext, Property};
//!
//! MockContext::guest()
//!     .with_property(Property::default())
//!     .with_capabilities(&["core.storage", "external.open-weather.pool"])
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
//! - [`SurfaceAssertions`] — SDUI tree helpers

#![deny(missing_docs)]

mod assertions;
mod fixtures;
mod mock_host;

pub use assertions::SurfaceAssertions;
pub use fixtures::{Booking, GuestIdentityFixture, Property};
pub use mock_host::{MockContext, MockContextBuilder, MockHostFunctions};
