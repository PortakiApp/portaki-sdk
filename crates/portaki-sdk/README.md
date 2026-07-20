<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://portaki.app/logo-dark.svg">
    <img src="https://portaki.app/logo-light.svg" width="177" height="48" alt="Portaki">
  </picture>
</p>

<h1 align="center">portaki-sdk</h1>

<p align="center">
  <strong>Authoring toolkit for Portaki Extism Wasm guest modules</strong><br>
  Host APIs, SDUI catalog, capability constants, and re-exported proc-macros.
</p>

<p align="center">
  <a href="https://crates.io/crates/portaki-sdk"><img src="https://img.shields.io/crates/v/portaki-sdk.svg" alt="crates.io"></a>
  <a href="https://docs.rs/portaki-sdk"><img src="https://img.shields.io/docsrs/portaki-sdk" alt="docs.rs"></a>
  <a href="https://github.com/PortakiApp/portaki-sdk/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-blue.svg" alt="License Apache-2.0"></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-1.75+-dea584?logo=rust&logoColor=white" alt="Rust 1.75+"></a>
  <a href="https://extism.org/"><img src="https://img.shields.io/badge/Extism-Wasm-7C3AED" alt="Extism"></a>
  <a href="https://portaki.app"><img src="https://img.shields.io/badge/site-portaki.app-f59e0b" alt="portaki.app"></a>
</p>

<p align="center">
  <a href="#install">Install</a> ·
  <a href="#quick-start">Quick start</a> ·
  <a href="#what-you-get">What you get</a> ·
  <a href="#workspace">Workspace</a> ·
  <a href="#documentation">Docs</a> ·
  <a href="#license">License</a>
</p>

---

This is the crate module authors depend on day to day. Proc-macros are re-exported from [`portaki-sdk-macros`](https://crates.io/crates/portaki-sdk-macros); the `portaki` CLI merges their compile-time emissions into `manifest.json` at build time.

## Install

```toml
[dependencies]
portaki-sdk = "0.1"
```

Guest builds target `wasm32-unknown-unknown`:

```bash
rustup target add wasm32-unknown-unknown
```

## Quick start

```rust,ignore
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

```bash
cargo install portaki-cli
portaki build --release
portaki lint
```

## What you get

| Surface | Role |
|---------|------|
| `host::*` | Typed host wrappers — KV, repo, connectors, events, i18n, geo, … |
| `sdui::*` | Surfaces, components, actions the shell can render |
| `capability::*` | Capability ids checked by the orchestrator and `portaki lint` |
| Proc-macros | `portaki_module!`, `#[surface]`, `#[query]`, `#[command]`, … |

## Workspace

| Crate | Role |
|-------|------|
| **portaki-sdk** | Runtime APIs + SDUI (this crate) |
| [`portaki-sdk-macros`](https://crates.io/crates/portaki-sdk-macros) | Manifest emissions at compile time |
| [`portaki-connectors`](https://crates.io/crates/portaki-connectors) | Typed built-in connector ops |
| [`portaki-test-utils`](https://crates.io/crates/portaki-test-utils) | In-process mock host for tests |
| [`portaki-cli`](https://crates.io/crates/portaki-cli) | `portaki` binary |

Monorepo: [`PortakiApp/portaki-sdk`](https://github.com/PortakiApp/portaki-sdk).

## Documentation

- API — [docs.rs/portaki-sdk](https://docs.rs/portaki-sdk)
- Connectors & credentials — [guide](https://github.com/PortakiApp/portaki-sdk/blob/main/docs/connectors-and-credentials.md)
- Releases — [RELEASE.md](https://github.com/PortakiApp/portaki-sdk/blob/main/docs/RELEASE.md)

## License

[Apache-2.0](https://github.com/PortakiApp/portaki-sdk/blob/main/LICENSE) · Copyright 2026 Syntax Labs
