# portaki-sdk

[![CI](https://github.com/PortakiApp/portaki-sdk/actions/workflows/ci.yml/badge.svg)](https://github.com/PortakiApp/portaki-sdk/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](./LICENSE)

Rust SDK, CLI, connectors, and test utilities for [Portaki](https://portaki.app) Wasm modules.

Portaki runs guest modules as Extism Wasm plugins. This workspace is what module authors use to build, lint, test, and publish OCI images to GitHub Container Registry (GHCR).

## Workspace crates

| Crate | Purpose |
|-------|---------|
| [`portaki-sdk`](./crates/portaki-sdk) | Host function wrappers, SDUI catalog, capability constants |
| [`portaki-sdk-macros`](./crates/portaki-sdk-macros) | Proc-macros that emit manifest metadata at compile time |
| [`portaki-connectors`](./crates/portaki-connectors) | Typed connectors (OpenWeather, Google Places, Mapbox, OSM, …) |
| [`portaki-test-utils`](./crates/portaki-test-utils) | `MockContext`, in-memory host functions, SDUI assertions |
| [`portaki-cli`](./crates/portaki-cli) | `portaki` binary — `init`, `build`, `lint`, `test`, `publish`, … |

## Requirements

- Rust **1.75+** (see `rust-version` in the root `Cargo.toml`)
- `wasm32-unknown-unknown` target for module builds:

```bash
rustup target add wasm32-unknown-unknown
```

## Install the CLI

```bash
cargo install --git https://github.com/PortakiApp/portaki-sdk --branch main --locked portaki-cli
```

## Quick start

```bash
cargo build --workspace
cargo test --workspace

cargo run -p portaki-cli -- init my-module --template default
cd my-module
portaki build --release
portaki lint
```

## Publish (OCI / GHCR)

After `portaki build --release`, push to GHCR. Image names use a dash — not a slash:

`ghcr.io/portakiapp/portaki-modules-<module-id>:<semver>`

```bash
export GITHUB_TOKEN="<classic-pat-with-write:packages>"   # or use docker login
export PORTAKI_PUBLISH_VERSION="0.2.1"                    # optional CI guard
portaki publish --registry ghcr.io/portakiapp
```

`docker login ghcr.io` also works — credentials are read from `~/.docker/config.json`.

`portaki publish --dry-run` validates the artifact layout without pushing.

When both `portaki.module.json` (host catalog) and SDK emissions exist, publish pushes two layers:

- `application/vnd.portaki.manifest+json` — frozen host catalog (`publish-manifest.json`)
- `application/vnd.portaki.sdk.manifest+json` — SDK emissions (`manifest.json`)

Official modules live in [`portaki-modules`](https://github.com/PortakiApp/portaki-modules).

## Related repositories

| Repository | Role |
|------------|------|
| [portaki-modules](https://github.com/PortakiApp/portaki-modules) | Official Wasm modules monorepo |
| [portaki-platform](https://github.com/PortakiApp/portaki-platform) | Orchestrator + module runtime |

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md). Please read [SECURITY.md](./SECURITY.md) before reporting vulnerabilities.

## License

Apache-2.0 — see [LICENSE](./LICENSE).

Copyright 2026 Syntax Labs.
