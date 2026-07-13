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
cd my-module
portaki build --release
portaki lint
```

## Publish (OCI)

After `portaki build --release`, push to GitHub Container Registry:

```bash
export GITHUB_TOKEN="<github-pat-with-write:packages>"
export PORTAKI_PUBLISH_VERSION="0.2.1"          # optional: fail if manifest version mismatches (CI sets from git tag)
portaki publish --registry ghcr.io/portakiapp/portaki-modules
```

Alternatively, use `docker login ghcr.io` — credentials are read from `~/.docker/config.json`.

`portaki publish --dry-run` validates `target/portaki/publish-manifest.json`, wasm, optional SDK manifest, and i18n bundles without pushing.

When both `portaki.module.json` (host catalog) and SDK emissions exist, publish pushes two manifest layers:

- `application/vnd.portaki.manifest+json` — frozen `publish-manifest.json` (API, install, config schema)
- `application/vnd.portaki.sdk.manifest+json` — `target/portaki/manifest.json` (surfaces, capabilities, i18n keys)

CI may fall back to `oras push` if `portaki publish` fails; layer media types match the ORAS layout.

## Macros (Phase 4)

- `#[capability(required, id = "core.storage")]` — `id` required when the const value is not a string literal.
- `#[entity_indexes(Entity)]` on `&["lat", "lng"]` — emits spatial index JSON for lat/lng pairs.
- `Temperature::variant(TempVariant::Hero | Inline | Compact)` — optional SDUI layout hint for guest shells.

## License

Apache-2.0
