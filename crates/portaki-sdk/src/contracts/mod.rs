//! SDK-owned **cross-module / platform** contracts.
//!
//! Use these when the wire name — or the whole payload shape — is shared across
//! modules or owned by the platform/shell. Never invent ad-hoc strings for peer
//! protocols, and never restate a shared payload shape in a module crate.
//!
//! Module-private surfaces, commands, and emit names stay in the module crate
//! via [`crate::define_surface_ids!`] / [`crate::define_operation_names!`] /
//! [`crate::define_event_types!`].

pub mod booking_channel;
pub mod host_fragments;
pub mod platform;
pub mod shell;
pub mod smart_lock;
pub mod stay_import;
