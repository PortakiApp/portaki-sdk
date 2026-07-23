//! SDK-owned **cross-module / platform** contracts.
//!
//! Use these when the wire name is shared across modules or owned by the
//! platform/shell — never invent ad-hoc strings for peer protocols.
//!
//! Module-private surfaces, commands, and emit names stay in the module crate
//! via [`crate::define_surface_ids!`] / [`crate::define_operation_names!`] /
//! [`crate::define_event_types!`].

pub mod platform;
pub mod shell;
pub mod smart_lock;
