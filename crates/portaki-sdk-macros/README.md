# portaki-sdk-macros

Proc-macros for Portaki Wasm modules.

These macros do **not** replace hand-written manifest files. At compile time they emit JSON fragments under `OUT_DIR/portaki-emissions/`. The [`portaki`](https://crates.io/crates/portaki-cli) CLI merges those fragments into `manifest.json` during `portaki build`.

Module authors normally depend on [`portaki-sdk`](https://crates.io/crates/portaki-sdk), which re-exports every macro in this crate. Direct dependency on `portaki-sdk-macros` is rarely needed.

## Install

Prefer the SDK re-export:

```toml
[dependencies]
portaki-sdk = "0.1"
```

## What gets emitted

| Macro | Emission kind (illustrative) |
|-------|------------------------------|
| `portaki_module!` / `#![portaki_module]` | Module identity, version, metadata |
| `#[entity]` / `#[entity_indexes]` | Repo entity shapes / indexes |
| `#[surface]` | Host/guest SDUI surfaces |
| `#[query]` / `#[command]` | Gateway operations |
| `#[event_handler]` | Event subscriptions |
| `#[capability]` | Declared capability ids |
| `#[connector]` / `#[custom_connector]` / `#[connector_op]` | Connector bindings & custom ops |

Exact JSON shapes and attribute keys are documented on each macro in [docs.rs/portaki-sdk-macros](https://docs.rs/portaki-sdk-macros).

## Example

```rust,ignore
use portaki_sdk::prelude::*;

portaki_module!(
    id = "weather",
    name = "Weather",
    description = "Forecast module",
);

#[capability(id = "host::connectors::call")]
struct NeedsConnectors;

#[query(id = "weather.current")]
fn current(_ctx: &Context, _input: ()) -> Result<()> {
    Ok(())
}
```

## License

Apache-2.0
