//! # Portaki module SDK
//!
//! Authoring toolkit for Wasm modules executed by the Portaki gateway. This crate
//! is the **module-side contract**: typed wrappers around host imports, SDUI
//! primitives, capability identifiers, invocation context, and proc-macros that
//! emit `manifest.json` fragments at compile time.
//!
//! ## What the host guarantees
//!
//! - Every query, command, and surface invocation receives a fully populated
//!   [`Context`] (property, plan, locale, effective capabilities, correlation id).
//! - Host functions (`host::kv`, `host::repo`, `host::connectors`, …) are routed
//!   through the gateway — modules never talk to Postgres, Redis, or third-party
//!   APIs directly.
//! - Capability grants reflected in `Context::capabilities` are authoritative for
//!   the current invocation; optional manifest capabilities may be absent.
//!
//! ## What modules must not assume
//!
//! - **Secrets in Wasm**: never embed API keys. Use [`host::connectors`] for egress
//!   (the gateway injects pool/BYOK secrets at the boundary).
//! - **Raw HTTP / SQL**: blocked by platform policy; use connectors and `host::repo`.
//! - **Cross-invocation memory**: Wasm instances are ephemeral — persist via
//!   `host::kv` or entity storage.
//! - **Synchronous side effects**: host calls may fail (`PortakiError`); surface
//!   renderers should degrade gracefully when optional capabilities are missing.
//!
//! ## Typical authoring path
//!
//! 1. Declare the module with `portaki_module!` and register surfaces/queries/commands
//!    via proc-macros (`surface!`, `query!`, `command!`, …).
//! 2. Import [`prelude`] in handler modules.
//! 3. Return [`Surface`] trees from render functions; call `host::*` for storage,
//!    connectors, and logging.
//! 4. Gate premium behaviour with `ctx.has_capability(...)` or
//!    [`mod@capability`] constants.
//! 5. Prefer [`ids`] newtypes and [`contracts`] for cross-boundary names
//!    (surfaces, commands, events, peer ops) instead of free string literals.
//! 6. Keep Wasm crate folders strict: `guest/` / `host/` / `connectors` / `ids`
//!    (see repo `docs/module-layout.md`).
//!
//! ## Wasm target
//!
//! On `wasm32`, see [`wasm`] for Extism entry points (`portaki_query`,
//! `portaki_command`) and the `portaki_host_dispatch` ABI.

#![deny(missing_docs)]

pub mod capability;
pub mod context;
pub mod contracts;
pub mod email;
pub mod error;
pub mod host;
pub mod ids;
pub mod manifest;
pub mod sdui;

pub mod wasm;

/// Re-export for `inventory::submit!` in wasm handler registration (macro-generated).
///
/// Module authors do not call this directly — `query!` / `command!` / `surface!`
/// shims submit [`wasm::HandlerRegistration`] entries at link time.
pub use inventory;

pub use capability::CapabilityId;
pub use context::{
    Context, DisplayPreferences, GuestContext, GuestIdentity, HostContext, PlanInfo,
    PropertyContext, Quota, StayContext,
};
pub use email::{EmailContextArgs, EmailContextContribution, EmailTemplateKey};
pub use error::{PortakiError, Result};
pub use ids::{EventType, ModuleId, OperationName, SurfaceId};
pub use portaki_sdk_macros::{
    capability, command, connector, connector_op, custom_connector, entity, entity_indexes,
    event_handler, portaki_module_decl as portaki_module, query, surface,
};
pub use sdui::{
    action::{json_value, Action, EmptyArgs, NavigateTarget, OverlayArgs, OverlayPresentation},
    component::Component,
    surface::Surface,
};

/// Commonly used imports for module handler code.
///
/// Pull this in once per module crate (`use portaki_sdk::prelude::*;`) to get
/// context types, host namespaces, SDUI root types, proc-macros, and utility macros.
///
/// # Examples
///
/// ```ignore
/// use portaki_sdk::prelude::*;
/// use portaki_sdk::sdui::primitives::{Card, Stack, Text};
///
/// #[surface(guest, id = "home.card")]
/// fn render_home(ctx: GuestContext) -> Surface {
///     if !ctx.has_capability(capability::core::STORAGE) {
///         return Surface::new(Text::new().text("i18n:capability.missing"));
///     }
///     log_info!("rendering home", surface = "home.card");
///     Surface::new(Card::new().child(Stack::new().child(Text::new())))
/// }
/// ```
pub mod prelude {
    pub use crate::capability::{self, CapabilityId};
    pub use crate::context::{Context, GuestContext, HostContext, StayContext};
    pub use crate::contracts;
    pub use crate::email::{EmailContextArgs, EmailTemplateKey};
    pub use crate::error::{PortakiError, Result};
    pub use crate::host;
    pub use crate::ids::{self, EventType, ModuleId, OperationName, SurfaceId};
    pub use crate::sdui::action::{
        json_value, Action, EmptyArgs, NavigateTarget, OverlayArgs, OverlayPresentation,
    };
    pub use crate::sdui::common::{
        ButtonVariant, ChoiceListLayout, ChoiceOption, Emphasis, GeoPoint, MapInteractionMode,
        MapMarker, MapMarkerKind, MapViewport, StackDirection, TempVariant, TemperatureUnit,
        TextVariant, Tone,
    };
    pub use crate::sdui::component::Component;
    pub use crate::sdui::surface::Surface;
    pub use crate::{
        command, connector, connector_op, custom_connector, define_event_types,
        define_operation_names, define_surface_ids, entity, entity_indexes, event_handler,
        portaki_module, query, surface,
    };
    pub use crate::{log_info, t};
    pub use chrono::{DateTime, Utc};
    pub use serde::{Deserialize, Serialize};
    pub use uuid::Uuid;
}

/// Resolves an i18n key through the host translation service.
///
/// Expands to [`host::i18n::translate`] with an empty or interpolated [`host::i18n::Vars`]
/// map. The active locale comes from the invocation [`Context`] — do not hard-code
/// locale strings in module UI copy.
///
/// Returns [`Result<String>`](crate::error::Result) because the host backend may be
/// unavailable in unit tests without [`host::runtime::with_host`].
///
/// # Examples
///
/// ```no_run
/// # use portaki_sdk::t;
/// let title = t!("module.home.title")?;
/// let greeting = t!("module.greeting", name = "Marie")?;
/// # Ok::<(), portaki_sdk::PortakiError>(())
/// ```
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

/// Emits a structured `info`-level log line to platform observability.
///
/// Field keys are taken from identifier tokens (`user_id = id` → `"user_id"`).
/// Values must implement `Serialize`. Prefer stable, low-cardinality field names
/// so logs aggregate cleanly in the gateway pipeline.
///
/// Silently no-ops field insertion when serialization fails for a single value.
///
/// # Examples
///
/// ```no_run
/// # use portaki_sdk::log_info;
/// log_info!("weather cache refreshed", property_id = "abc", ttl_secs = 300);
/// ```
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
