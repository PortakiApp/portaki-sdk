//! Server-Driven UI catalog — typed primitives and actions.

pub mod action;
pub mod common;
pub mod component;
pub mod primitives;
pub mod surface;

pub use action::Action;
pub use common::{Animation, Emphasis, SurfaceLevel, Tone, Visibility};
pub use component::Component;
pub use surface::Surface;
