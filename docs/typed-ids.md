# Typed boundary identifiers

SDK newtypes are **required** when a name crosses a Wasm / SDUI / host / peer-module
boundary. Wire format stays a JSON string (`AsRef<str>` / serde transparent).

## Rule

1. **Declare once** — string literal only in `define_*!`, `#[surface(id = …)]` /
   `#[command(name = …)]` / `#[event_handler(event_type = …)]`, or `Type::new` /
   `ModuleId::from_static` in tests and define macros.
2. **Use typed consts** everywhere else — `ids::HOME_CARD`, `UPDATE_CONFIG`,
   `contracts::shell::SURFACE_INPUT`, never inline `"home.card"` at call sites.

## Catalogs

| Type | Use for | Closed? |
|------|---------|---------|
| [`CapabilityId`](../crates/portaki-sdk/src/capability.rs) | Plan / connector grants, `has_capability` | Yes |
| [`EmailTemplateKey`](../crates/portaki-sdk/src/email.rs) | Transactional email templates | Yes |
| [`SurfaceId`](../crates/portaki-sdk/src/ids.rs) | Manifest surfaces, `Navigate` (surface), `OpenOverlay`, `Surface::with_id` | No — per module (+ [`convention`](../crates/portaki-sdk/src/ids.rs)) |
| [`OperationName`](../crates/portaki-sdk/src/ids.rs) | Command / query names, peer ops | No — except [`contracts`](../crates/portaki-sdk/src/contracts) |
| [`ModuleId`](../crates/portaki-sdk/src/ids.rs) | `Action::command`, peer discovery, `Context::module_id` | No |
| [`EventType`](../crates/portaki-sdk/src/ids.rs) | `events::emit`, `Action::Emit`, `#[event_handler]` | Partial — platform/shell in contracts |
| [`NavigateTarget`](../crates/portaki-sdk/src/sdui/action.rs) | `Action::navigate` — `Surface(SurfaceId)` or `Path(String)` | — |

## Module-local catalogs

```rust,ignore
use portaki_sdk::prelude::*;

define_surface_ids! {
    HOME_CARD = "home.card",
    EXPLORE_FORECAST = "explore.forecast",
    HOST_MAIN = "main",
}

define_operation_names! {
    UPDATE_CONFIG = "updateConfig",
    REFRESH = "refreshForecast",
}

#[surface(guest, id = "home.card")] // declaration site — literal OK once
fn render_home(ctx: GuestContext) -> Surface {
    Surface::new(/* … */)
        .with_id(HOME_CARD)
}

// Runtime actions — typed consts only:
Action::open_overlay(OverlayPresentation::BottomSheet, EXPLORE_FORECAST, None);
Action::command(&ctx.module_id, UPDATE_CONFIG, EmptyArgs {});
Action::navigate(HOME_CARD, None);
Action::navigate(NavigateTarget::path(format!("appliances/{id}")), None);
```

## Cross-module contracts (SDK-owned)

```rust,ignore
use portaki_sdk::contracts::{platform, shell, smart_lock};
use portaki_sdk::prelude::*;

let peers = host::module::list_by_capability(smart_lock::CAPABILITY)?;
Action::command(&peers[0].module_id, smart_lock::UNLOCK, args);

Action::emit(shell::SURFACE_INPUT, Some(payload));

#[event_handler(event_type = "core.booking.confirmed")] // declaration site
fn on_booking(ctx: Context, event: BookingConfirmedEvent) -> Result<()> { /* … */ }
let _ = platform::BOOKING_CONFIRMED;
```

Do **not** invent a monorepo-wide enum of every module’s private surfaces.
Do keep peer protocols (`access.smart_lock`, shell events, `core.*` platform
events) in the SDK so consumers cannot mistype the contract.
