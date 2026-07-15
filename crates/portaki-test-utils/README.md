<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://portaki.app/logo-dark.svg">
    <img src="https://portaki.app/logo-light.svg" width="177" height="48" alt="Portaki">
  </picture>
</p>

<h1 align="center">portaki-test-utils</h1>

<p align="center">
  <strong>In-process mock host for Portaki module unit tests</strong><br>
  <code>MockContext</code>, in-memory host functions, and SDUI assertions — no Wasm, no Extism.
</p>

<p align="center">
  <a href="https://crates.io/crates/portaki-test-utils"><img src="https://img.shields.io/crates/v/portaki-test-utils.svg" alt="crates.io"></a>
  <a href="https://docs.rs/portaki-test-utils"><img src="https://img.shields.io/docsrs/portaki-test-utils" alt="docs.rs"></a>
  <a href="https://github.com/PortakiApp/portaki-sdk/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-blue.svg" alt="License Apache-2.0"></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-1.75+-dea584?logo=rust&logoColor=white" alt="Rust 1.75+"></a>
  <a href="https://portaki.app"><img src="https://img.shields.io/badge/site-portaki.app-f59e0b" alt="portaki.app"></a>
</p>

<p align="center">
  <a href="#install">Install</a> ·
  <a href="#quick-start">Quick start</a> ·
  <a href="#what-you-get">What you get</a> ·
  <a href="#documentation">Docs</a> ·
  <a href="#license">License</a>
</p>

---

Dev-dependency for module crates. Production code calls `portaki_sdk::host::*`; this crate supplies a [`HostBackend`](https://docs.rs/portaki-sdk/latest/portaki_sdk/host/trait.HostBackend.html) that stays entirely in memory on the test thread.

## Install

```toml
[dev-dependencies]
portaki-sdk = "0.1"
portaki-test-utils = "0.1"
```

## Quick start

```rust,ignore
use portaki_test_utils::{MockContext, Property, SurfaceAssertions};

#[test]
fn guest_home_renders() {
    MockContext::guest()
        .with_property(Property::default())
        .with_capabilities(&["core.storage"])
        .run(|ctx| {
            let surface = guest_home(ctx).expect("render");
            SurfaceAssertions::new(&surface);
        });
}
```

Stub connectors used by [`portaki-connectors`](https://crates.io/crates/portaki-connectors):

```rust,ignore
MockContext::guest()
    .with_connector_response(
        "open-weather",
        "current",
        r#"{"main":{"temp":21.5},"weather":[{"main":"Clear"}]}"#,
    )
    .run(|_ctx| { /* OpenWeather::current reads the stub */ });
```

## What you get

| Type | Role |
|------|------|
| `MockContext` / `MockContextBuilder` | Fluent guest/host context + backend install |
| `MockHostFunctions` | In-memory KV, i18n, connectors, repo stubs |
| `Property`, `Booking`, … | Default fixtures |
| `SurfaceAssertions` | SDUI tree helpers |

## Documentation

- API — [docs.rs/portaki-test-utils](https://docs.rs/portaki-test-utils)
- Workspace — [`PortakiApp/portaki-sdk`](https://github.com/PortakiApp/portaki-sdk)

## License

[Apache-2.0](https://github.com/PortakiApp/portaki-sdk/blob/main/LICENSE) · Copyright 2026 Syntax Labs
