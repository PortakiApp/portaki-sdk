<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://portaki.app/logo-dark.svg">
    <img src="https://portaki.app/logo-light.svg" width="177" height="48" alt="Portaki">
  </picture>
</p>

<h1 align="center">portaki-connectors</h1>

<p align="center">
  <strong>Typed built-in connector operations for Portaki modules</strong><br>
  OpenWeather, Mapbox, Google Places, OSM Nominatim — shapes & ops, host does the HTTP.
</p>

<p align="center">
  <a href="https://crates.io/crates/portaki-connectors"><img src="https://img.shields.io/crates/v/portaki-connectors.svg" alt="crates.io"></a>
  <a href="https://docs.rs/portaki-connectors"><img src="https://img.shields.io/docsrs/portaki-connectors" alt="docs.rs"></a>
  <a href="https://github.com/PortakiApp/portaki-sdk/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-blue.svg" alt="License Apache-2.0"></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-1.75+-dea584?logo=rust&logoColor=white" alt="Rust 1.75+"></a>
  <a href="https://portaki.app"><img src="https://img.shields.io/badge/site-portaki.app-f59e0b" alt="portaki.app"></a>
</p>

<p align="center">
  <a href="#install">Install</a> ·
  <a href="#providers">Providers</a> ·
  <a href="#example">Example</a> ·
  <a href="#credentials">Credentials</a> ·
  <a href="#documentation">Docs</a> ·
  <a href="#license">License</a>
</p>

---

Each type maps to a connector id known by the orchestrator. Modules call through `portaki_sdk::host::connectors::call` — this crate only provides **request/response shapes** and **operation names**. It does not open sockets; the Wasm host performs egress (pool or BYOK).

## Install

```toml
[dependencies]
portaki-sdk = "0.1"
portaki-connectors = "0.1"
```

## Providers

| Module | Connector id | Operations |
|--------|--------------|------------|
| `open_weather` | `open-weather` | `current`, `forecast`, `historical` |
| `google_places` | `google-places` | `nearby_search`, `text_search`, `details`, `photos` |
| `mapbox` | `mapbox` | `geocode`, `reverse_geocode`, `directions`, `static_map` |
| `osm_nominatim` | `osm-nominatim` | `geocode`, `reverse_geocode` |

## Example

```rust,ignore
use portaki_connectors::open_weather::{CurrentArgs, OpenWeather};
use portaki_sdk::host;

let weather = OpenWeather::current(&CurrentArgs { lat: 43.55, lng: 7.01 })?;
```

In unit tests, stub JSON with [`portaki-test-utils`](https://crates.io/crates/portaki-test-utils) (`MockContextBuilder::with_connector_response`).

## Credentials

Pool + BYOK wiring lives on custom-connector macros and the orchestrator. Author guide:

[connectors-and-credentials.md](https://github.com/PortakiApp/portaki-sdk/blob/main/docs/connectors-and-credentials.md)

## Documentation

- API — [docs.rs/portaki-connectors](https://docs.rs/portaki-connectors)
- Workspace — [`PortakiApp/portaki-sdk`](https://github.com/PortakiApp/portaki-sdk)

## License

[Apache-2.0](https://github.com/PortakiApp/portaki-sdk/blob/main/LICENSE) · Copyright 2026 Syntax Labs
