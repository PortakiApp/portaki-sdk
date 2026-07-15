# portaki-connectors

Typed **built-in connector** operations for Portaki modules.

Each type maps to a connector id known by the orchestrator (OpenWeather, Mapbox, Google Places, OSM Nominatim, …). Modules invoke them through `portaki_sdk::host::connectors::call` — this crate only provides request/response shapes and operation names. It does **not** perform HTTP itself; the Wasm host does.

## Install

```toml
[dependencies]
portaki-sdk = "0.1"
portaki-connectors = "0.1"
```

## Example

```rust,ignore
use portaki_connectors::OpenWeather;
use portaki_sdk::host::connectors;

let weather = connectors::call::<OpenWeather::Current>(/* … */)?;
```

See [docs.rs/portaki-connectors](https://docs.rs/portaki-connectors) for the full operation table per provider.

Credential pool / BYOK wiring lives on the custom-connector macros and the orchestrator — see [connectors-and-credentials.md](https://github.com/PortakiApp/portaki-sdk/blob/main/docs/connectors-and-credentials.md).

## License

Apache-2.0
