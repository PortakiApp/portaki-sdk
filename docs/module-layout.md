# Module & SDK source layout

Strict folder conventions so concerns do not mix. Wire format / serde names stay
unchanged — this is organization only.

Related: [typed-ids.md](./typed-ids.md), [connectors-and-credentials.md](./connectors-and-credentials.md).

---

## SDK crate (`crates/portaki-sdk`)

Top-level modules map 1:1 to authoring concerns:

| Module | Responsibility |
|--------|----------------|
| [`capability`](../crates/portaki-sdk/src/capability.rs) | Closed `CapabilityId` catalog + plan grants |
| [`ids`](../crates/portaki-sdk/src/ids.rs) | Boundary newtypes (`SurfaceId`, `OperationName`, …) + `define_*!` |
| [`contracts`](../crates/portaki-sdk/src/contracts/) | Cross-module / platform / shell catalogs |
| [`context`](../crates/portaki-sdk/src/context.rs) | Invocation `Context` / `GuestContext` / `HostContext` |
| [`email`](../crates/portaki-sdk/src/email.rs) | Email context contribution types + template keys |
| [`error`](../crates/portaki-sdk/src/error.rs) | `PortakiError` / `Result` |
| [`manifest`](../crates/portaki-sdk/src/manifest.rs) | Manifest fragment types for CLI / codegen |
| [`sdui`](../crates/portaki-sdk/src/sdui/) | **Shared** SDUI primitives, actions, surfaces |
| [`host`](../crates/portaki-sdk/src/host/) | Host import wrappers (`kv`, `repo`, `connectors`, …) |
| [`wasm`](../crates/portaki-sdk/src/wasm/) | Extism entry / dispatch / registry (Wasm runtime) |

Sibling crates (same workspace, different concern):

| Crate | Responsibility |
|-------|----------------|
| `portaki-sdk-macros` | Proc-macros that emit manifest fragments |
| `portaki-connectors` | Typed connector request/response DTOs used by modules |
| `portaki-cli` | `portaki` binary (build / lint / publish) |
| `portaki-test-utils` | Host mocks / fixtures for module tests |

### SDUI: shared primitives, shell-split at registration

There is **no** `sdui::host` / `sdui::guest` type split. Components, tokens, and
`Action` are shared. Host vs guest differs only when you register a surface:

```rust,ignore
#[surface(guest, id = "home.card")]
fn render_home(ctx: GuestContext) -> Surface { /* … */ }

#[surface(host, id = "main")]
fn render_host_main(ctx: HostContext) -> Surface { /* … */ }
```

Do not put host-dashboard-only helpers inside guest booklet modules in a Wasm
crate (see module layout below). In the SDK, keep SDUI catalog code under
`sdui/` and host **imports** under `host/`.

### Public API / prelude

- Crate root re-exports a **small** set of everyday types + macros.
- [`prelude`](../crates/portaki-sdk/src/lib.rs) is intentional authoring sugar
  (context, host namespace, SDUI roots, `define_*!`, serde/chrono/uuid) — not a
  dump of every submodule.
- Prefer `use portaki_sdk::sdui::primitives::{Card, Text}` (or prelude +
  primitives) over deep internal paths.
- Prefer `use portaki_sdk::host::kv` over re-exporting every host op at root.

### Idiomatic Rust

- One concern per file / small `mod` tree; no god files that mix SDUI + host
  ABI + connectors.
- File-per-type when it clarifies (`contracts/smart_lock.rs`); `mod.rs` for
  thin module trees.
- Explicit types; no drive-by refactors of wire shapes.

---

## Wasm module crates (`portaki-modules` / third-party)

Canonical layout (omit folders you do not need — do not invent empty layers):

```
src/
  lib.rs              # portaki_module! + capability consts (+ thin pub use)
  ids.rs              # define_surface_ids! / define_operation_names! / define_event_types!
  guest/              # guest SDUI surfaces only (#[surface(guest, …)])
  host/               # host SDUI surfaces only (#[surface(host, …)])
  connectors/         # connector! / connector_op! (or connectors.rs if tiny)
  commands.rs         # #[command] handlers (or commands/)
  queries.rs          # #[query] handlers (or queries/)
  model/ or entities  # domain types / entity! structs (pick one name per crate)
  config.rs           # persisted module config load/store helpers
  email_context.rs    # email contribution (or email/)
  events.rs           # #[event_handler] (optional)
  i18n/               # locale bundles (repo convention: crate-root i18n/, not src/)
```

### Hard rules

1. **Guest SDUI, host SDUI, connectors, commands/queries, and domain types must
   not share one file.**
2. `lib.rs` stays thin: `portaki_module!`, `#[capability]` consts, `mod` lines,
   and optional `pub use` for the crate’s public / test API. No surface bodies.
3. Rename legacy `render_host.rs` → `host/` (usually `host/mod.rs` with
   `render_host_main`).
4. Put typed id catalogs in `ids.rs` — see [typed-ids.md](./typed-ids.md). Prefer
   `ids::convention` / local consts over bare `"home.card"` at call sites.
5. Connectors live in `connectors` (module) or `portaki-connectors` (shared DTO
   crate) — never inside a guest surface file.

### Allowed exceptions

| Case | Guidance |
|------|----------|
| Guest-only module (e.g. static content, no host editor) | Omit `host/` |
| No connectors | Omit `connectors` |
| Tiny command surface (one fn) | Keep `commands.rs` at `src/` root |
| Domain folder already named `entities` / `weather` / `content` | Keep the name; do not rename for fashion |
| Shared guest helpers (`load.rs`, `body.rs`) | Stay under `guest/` |

### Example `lib.rs`

```rust,ignore
//! Short module blurb.

mod commands;
mod config;
mod connectors;
mod email_context;
mod entities;
mod guest;
mod host;
mod ids;
mod queries;

pub use commands::update_config;
pub use guest::{render_explore_forecast, render_home_card};
pub use host::render_host_main;
pub use queries::get_forecast;

portaki_sdk::portaki_module!(
    id = "weather",
    display_name_key = "module.displayName",
    description_key = "module.description",
    author = "Portaki",
);

#[portaki_sdk::capability(required, id = "core.storage")]
pub const STORAGE: portaki_sdk::CapabilityId = portaki_sdk::capability::core::STORAGE;
```

### Example `ids.rs`

```rust,ignore
use portaki_sdk::prelude::*;

define_surface_ids! {
    HOME_CARD = "home.card",
    EXPLORE_FORECAST = "explore.forecast",
    HOST_MAIN = "main",
}

define_operation_names! {
    UPDATE_CONFIG = "updateConfig",
    GET_FORECAST = "getForecast",
    REFRESH_FORECAST = "refreshForecast",
}
```

---

## Checklist (PR)

- [ ] No new host surface landed in `guest/`
- [ ] No connector / command / entity types jammed into a surface file
- [ ] `ids.rs` catalogs cover surfaces and ops touched by the change
- [ ] Prelude imports stay intentional — no wildcard re-exports of private helpers
- [ ] Wire strings / serde renames unchanged
