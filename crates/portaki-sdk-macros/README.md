<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://portaki.app/logo-dark.svg">
    <img src="https://portaki.app/logo-light.svg" width="177" height="48" alt="Portaki">
  </picture>
</p>

<h1 align="center">portaki-sdk-macros</h1>

<p align="center">
  <strong>Proc-macros for Portaki Wasm modules</strong><br>
  Compile-time JSON emissions merged into <code>manifest.json</code> by <code>portaki build</code>.
</p>

<p align="center">
  <a href="https://crates.io/crates/portaki-sdk-macros"><img src="https://img.shields.io/crates/v/portaki-sdk-macros.svg" alt="crates.io"></a>
  <a href="https://docs.rs/portaki-sdk-macros"><img src="https://img.shields.io/docsrs/portaki-sdk-macros" alt="docs.rs"></a>
  <a href="https://github.com/PortakiApp/portaki-sdk/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-blue.svg" alt="License Apache-2.0"></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-1.75+-dea584?logo=rust&logoColor=white" alt="Rust 1.75+"></a>
  <a href="https://portaki.app"><img src="https://img.shields.io/badge/site-portaki.app-f59e0b" alt="portaki.app"></a>
</p>

<p align="center">
  <a href="#install">Install</a> ·
  <a href="#what-gets-emitted">Emissions</a> ·
  <a href="#example">Example</a> ·
  <a href="#documentation">Docs</a> ·
  <a href="#license">License</a>
</p>

---

Module authors normally depend on [`portaki-sdk`](https://crates.io/crates/portaki-sdk), which **re-exports** every macro here. A direct dependency on `portaki-sdk-macros` is rarely needed.

At compile time, each macro writes a JSON fragment under:

```text
{OUT_DIR}/portaki-emissions/{kind}-{sanitized_key}.json
```

[`portaki build`](https://crates.io/crates/portaki-cli) discovers the newest emissions directory and merges them into `target/portaki/manifest.json`.

## Install

```toml
[dependencies]
portaki-sdk = "0.1"   # preferred — re-exports these macros
```

## What gets emitted

| Macro | Kind (illustrative) |
|-------|---------------------|
| `portaki_module!` / `#![portaki_module]` | Module identity & version |
| `#[entity]` / `#[entity_indexes]` | Repo entity shapes / indexes |
| `#[surface]` | Host / guest SDUI surfaces |
| `#[query]` / `#[command]` | Gateway operations |
| `#[event_handler]` | Event subscriptions |
| `#[capability]` | Declared capability ids |
| `#[connector]` / `#[custom_connector]` / `#[connector_op]` | Connector bindings & custom ops |

Attribute keys, JSON shapes, and `compile_error` conditions are documented on each item in [docs.rs](https://docs.rs/portaki-sdk-macros).

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

## Documentation

- API — [docs.rs/portaki-sdk-macros](https://docs.rs/portaki-sdk-macros)
- Workspace — [`PortakiApp/portaki-sdk`](https://github.com/PortakiApp/portaki-sdk)

## License

[Apache-2.0](https://github.com/PortakiApp/portaki-sdk/blob/main/LICENSE) · Copyright 2026 Syntax Labs
