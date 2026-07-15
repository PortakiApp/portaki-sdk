//! Unified SDUI [`super::primitives::Component`] enum.
//!
//! Re-exported from generated primitives (`build.rs` / `sdui_primitives.json`).
//! Each struct variant (e.g. `Card`, `Stack`, `Text`) implements `Into<Component>`
//! so surface builders compose with `.child(...)` fluent APIs.
//!
//! ## Contract
//!
//! - Variant names and fields are schema-driven — add primitives in
//!   `sdui_primitives.json`, not by editing this module.
//! - Unknown variants fail shell deserialization — pin `ui_schema` versions when
//!   adopting new primitives.
//!
//! See [`super::primitives`] for concrete builder types.

pub use super::primitives::Component;
