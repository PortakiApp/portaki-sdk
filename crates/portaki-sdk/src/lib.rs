//! Portaki module SDK for authoring Wasm modules.
//!
//! Provides typed host function wrappers, the SDUI catalog, capability constants,
//! and re-exports of proc-macros used to emit manifest metadata at compile time.

#![deny(missing_docs)]

pub mod capability;
pub mod context;
pub mod error;
pub mod host;
pub mod manifest;
pub mod sdui;

pub mod wasm;

/// Re-export for `inventory::submit!` in wasm handler registration (macro-generated).
pub use inventory;

pub use context::{
    Context, DisplayPreferences, GuestContext, GuestIdentity, HostContext, PlanInfo,
    PropertyContext, Quota,
};
pub use error::{PortakiError, Result};
pub use portaki_sdk_macros::{
    capability, command, connector, connector_op, custom_connector, entity, entity_indexes,
    event_handler, portaki_module_decl as portaki_module, query, surface,
};
pub use sdui::{action::Action, component::Component, surface::Surface};

/// Re-export commonly used crates for module authors.
pub mod prelude {
    pub use crate::capability;
    pub use crate::context::{Context, GuestContext, HostContext};
    pub use crate::error::{PortakiError, Result};
    pub use crate::host;
    pub use crate::sdui::component::Component;
    pub use crate::sdui::surface::Surface;
    pub use crate::{
        command, connector, connector_op, custom_connector, entity, entity_indexes, event_handler,
        portaki_module, query, surface,
    };
    pub use crate::{log_info, t};
    pub use chrono::{DateTime, Utc};
    pub use serde::{Deserialize, Serialize};
    pub use uuid::Uuid;
}

/// Resolves an i18n key through the host i18n API (shorthand for modules).
#[macro_export]
macro_rules! t {
    ($key:expr) => {
        $crate::host::i18n::translate($key, &$crate::host::i18n::Vars::new())
    };
    ($key:expr, $($name:ident = $value:expr),+ $(,)?) => {{
        let mut vars = $crate::host::i18n::Vars::new();
        $( vars.set(stringify!($name), $value); )+
        $crate::host::i18n::translate($key, &vars)
    }};
}

/// Structured logging shorthand.
#[macro_export]
macro_rules! log_info {
    ($msg:expr $(, $key:ident = $value:expr)*) => {
        {
            let mut fields = $crate::host::log::Fields::new();
            $( fields.insert(stringify!($key), &$value); )*
            $crate::host::log::info($msg, &fields);
        }
    };
}
