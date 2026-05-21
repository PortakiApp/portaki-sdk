# portaki-sdk-rust

Rust SDK, CLI, connectors, and test utilities for [Portaki](https://github.com/syntax-labs) Wasm modules.

## Workspace crates

| Crate | Purpose |
|-------|---------|
| `portaki-sdk` | Host function wrappers, SDUI catalog, capability constants |
| `portaki-sdk-macros` | Proc-macros emitting manifest metadata at compile time |
| `portaki-connectors` | Typed built-in connectors (Google Places, Mapbox, OpenWeather, OSM) |
| `portaki-test-utils` | `MockContext`, in-memory host functions, SDUI assertions |
| `portaki-cli` | `portaki` binary (`init`, `build`, `lint`, `test`, `publish`, …) |

## Quick start

```bash
cargo build --workspace
cargo test --workspace
cargo run -p portaki-cli -- init my-module --template default
```

## Branch

Agent B1 work targets `feature/B1-sdk-cli`.

## License

Apache-2.0
