# portaki-sdk

Authoring toolkit for **Portaki Extism Wasm modules**.

This crate is what module authors depend on. It provides:

- Typed **host function** wrappers (`host::*`) — KV, repo, connectors, credentials, i18n, geo, …
- The **SDUI catalog** (`sdui::*`) — surfaces, components, actions the host can render
- **Capability** constants checked by the orchestrator and `portaki lint`
- Re-exported **proc-macros** from [`portaki-sdk-macros`](https://crates.io/crates/portaki-sdk-macros) that emit manifest metadata at compile time

## Install

```toml
[dependencies]
portaki-sdk = "0.1"
```

Target `wasm32-unknown-unknown` for guest builds:

```bash
rustup target add wasm32-unknown-unknown
```

## Quick start

```rust,ignore
#![allow(unused)]
use portaki_sdk::prelude::*;

portaki_module!(
    id = "weather",
    name = "Weather",
    description = "Current weather and forecast",
);

#[surface(id = "guest.home", audience = "guest")]
fn guest_home(_ctx: &Context) -> Result<Surface> {
    Ok(Surface::new("guest.home"))
}
```

Then build with the CLI:

```bash
cargo install portaki-cli
portaki build --release
portaki lint
```

## Crate map

| Crate | Role |
|-------|------|
| **portaki-sdk** | Runtime APIs + SDUI (this crate) |
| [`portaki-sdk-macros`](https://crates.io/crates/portaki-sdk-macros) | Proc-macros / manifest emissions |
| [`portaki-connectors`](https://crates.io/crates/portaki-connectors) | Typed built-in connector ops |
| [`portaki-test-utils`](https://crates.io/crates/portaki-test-utils) | In-process mock host for tests |
| [`portaki-cli`](https://crates.io/crates/portaki-cli) | `portaki` binary (build / lint / publish) |

## Documentation

- API: [docs.rs/portaki-sdk](https://docs.rs/portaki-sdk)
- Connectors & credentials: [docs/connectors-and-credentials.md](https://github.com/PortakiApp/portaki-sdk/blob/main/docs/connectors-and-credentials.md)
- Releases: [docs/RELEASE.md](https://github.com/PortakiApp/portaki-sdk/blob/main/docs/RELEASE.md)

## License

Apache-2.0
