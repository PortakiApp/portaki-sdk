# Typed boundary identifiers

SDK newtypes are **required** when a name crosses a Wasm / SDUI / host / peer-module
boundary. Wire format stays a JSON string (`AsRef<str>` / serde transparent).

## Rule

1. **Declare once** — the attribute is the declaration: `#[surface(…, id = "explore.detail")]`
   defines `EXPLORE_DETAIL: SurfaceId` next to the renderer, `#[query(name = "listSources")]` /
   `#[command(name = …)]` define `LIST_SOURCES: OperationName` next to the handler
   (`SCREAMING_SNAKE_CASE` of the wire string). The dispatcher stamps the declared id on the
   surface a renderer returns — no `.with_id(…)`. Events: `define_event_types!`. Anything else:
   `Type::new("…")` / `ModuleId::from_static`.
   Proc-macros need the wire string at expand time (OUT_DIR emissions) — they
   **cannot** take a bare `ids::CONST` path.
2. **Use typed consts** everywhere else — `guest::EXPLORE_DETAIL`, `LIST_SOURCES`,
   `contracts::shell::SURFACE_INPUT`, never inline `"home.card"` at call sites.
3. **No `ids.rs`** — `define_surface_ids!` / `define_operation_names!` are deprecated (still
   compile). Peer / platform protocols live in [`contracts`](../crates/portaki-sdk/src/contracts).

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
| [`BookingChannel`](../crates/portaki-sdk/src/contracts/booking_channel.rs) | Who sold an imported stay — host platform selectors, `StayImportRow` | Yes |
| [`ChannelSignal`](../crates/portaki-sdk/src/contracts/booking_channel.rs) | How a `BookingChannel` was established | Yes |

## Module-local ids

```rust,ignore
use portaki_sdk::prelude::*;

// guest/mod.rs — declares `HOME_CARD` and `EXPLORE_FORECAST`
#[surface(guest, id = "home.card")]
fn render_home(ctx: GuestContext) -> Surface {
    Surface::new(/* … */) // the dispatcher stamps "home.card"
}

#[surface(guest, id = "explore.forecast")]
fn render_forecast(ctx: GuestContext) -> Surface { /* … */ }

// commands.rs — declares `REFRESH_FORECAST`
#[command(name = "refreshForecast")]
fn refresh(ctx: Context) -> Result<()> { /* … */ }

// Runtime actions — typed consts only:
Action::open_overlay(OverlayPresentation::BottomSheet, guest::EXPLORE_FORECAST, None);
Action::command(&ctx.module_id, commands::REFRESH_FORECAST, EmptyArgs {});
Action::navigate(guest::HOME_CARD, None);
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
// Prefer `contracts::platform::BOOKING_CONFIRMED` (or mirrored `ids::BOOKING_CONFIRMED`)
// at any non-macro use site.
let _ = platform::BOOKING_CONFIRMED;
```

Do **not** invent a monorepo-wide enum of every module’s private surfaces.
Do keep peer protocols (`access.smart_lock`, shell events, `core.*` platform
events) in the SDK so consumers cannot mistype the contract.

## Shared payload shapes

`contracts` also owns payload shapes the gateway parses from more than one
module — `stay_import::StayImportRow` for any calendar / channel-manager import
module. Emit the SDK struct instead of restating the field set in the module
crate; a shape held together by a comment drifts as soon as a second module
ships.
