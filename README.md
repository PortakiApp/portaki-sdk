<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://portaki.app/logo-dark.svg">
    <img src="https://portaki.app/logo-light.svg" width="177" height="48" alt="Portaki">
  </picture>
</p>

<h1 align="center">portaki-sdk</h1>

<p align="center">
  <strong>Rust SDK, CLI, connectors, and test utilities for Portaki Wasm guest modules</strong><br>
  Build, lint, test, and release Extism modules to the Portaki registry.
</p>

<p align="center">
  <a href="https://github.com/PortakiApp/portaki-sdk/actions/workflows/ci.yml"><img src="https://github.com/PortakiApp/portaki-sdk/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-blue.svg" alt="License Apache-2.0"></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-1.75+-dea584?logo=rust&logoColor=white" alt="Rust 1.75+"></a>
  <a href="https://extism.org/"><img src="https://img.shields.io/badge/Extism-Wasm-7C3AED" alt="Extism"></a>
  <a href="https://portaki.app"><img src="https://img.shields.io/badge/site-portaki.app-f59e0b" alt="portaki.app"></a>
</p>

<p align="center">
  <a href="#install">Install</a> ·
  <a href="#quick-start">Quick start</a> ·
  <a href="#release">Release</a> ·
  <a href="#workspace-crates">Crates</a> ·
  <a href="docs/connectors-and-credentials.md">Connectors & credentials</a> ·
  <a href="docs/module-layout.md">Module layout</a> ·
  <a href="docs/typed-ids.md">Typed ids</a> ·
  <a href="docs/host-surfaces-and-contracts.md">Host surfaces & contracts</a> ·
  <a href="docs/guest-operations.md">Guest operations</a> ·
  <a href="CONTRIBUTING.md">Contributing</a> ·
  <a href="SECURITY.md">Security</a>
</p>

---

Portaki runs guest modules as **Extism Wasm** plugins. This workspace is what module authors use day to day: host APIs, proc-macros, connectors, mocks, and the `portaki` CLI.

Official modules are published from [`portaki-modules`](https://github.com/PortakiApp/portaki-modules)
to Portaki's own OCI repository, as `oci.portaki.app/modules/<module-id>:<semver>`.

## Why this SDK?

- **One toolchain** — `portaki init` / `build` / `lint` / `test` / `release`
- **Compile-time metadata** — macros emit catalog + SDK manifests consumed by the host
- **Connectors** — typed clients for OpenWeather, Google Places, Mapbox, OSM, …
- **Testable** — `MockContext` and in-memory host functions without a full runtime
- **OCI-native** — `portaki release` pushes to Portaki's OCI repository, signs, and announces

## Connectors and credentials

Declare pool + BYOK with `#[custom_connector(... credential_provider_id = "...")]` and `external.<provider>.pool` / `.byok` capabilities. The orchestrator derives which module needs which key from the published manifest — no dashboard hardcoding.

See **[docs/connectors-and-credentials.md](./docs/connectors-and-credentials.md)** (weather is the reference module).

## Workspace crates

| Crate | Purpose |
|-------|---------|
| [`portaki-sdk`](./crates/portaki-sdk) | Host function wrappers, SDUI catalog, capability constants |
| [`portaki-sdk-macros`](./crates/portaki-sdk-macros) | Proc-macros that emit manifest metadata at compile time |
| [`portaki-connectors`](./crates/portaki-connectors) | Typed external connectors |
| [`portaki-test-utils`](./crates/portaki-test-utils) | `MockContext`, in-memory host functions, SDUI assertions |
| [`portaki-cli`](./crates/portaki-cli) | `portaki` binary |

## Requirements

- Rust **1.75+**
- Target `wasm32-unknown-unknown` for module builds

```bash
rustup target add wasm32-unknown-unknown
```

## Install

```bash
cargo install --git https://github.com/PortakiApp/portaki-sdk --branch main --locked portaki-cli
```

Credentials live in `~/.config/portaki/credentials.json` (`0600`), not in the system keychain —
see [docs/cli-keychain-macos.md](docs/cli-keychain-macos.md) for what that buys, what it does
not, and how to switch back.

## Quick start

```bash
cargo build --workspace
cargo test --workspace

cargo run -p portaki-cli -- init my-module --template default
cd my-module
portaki check
```

## Release

```bash
portaki login
portaki release            # tests, build, push, sign, announce
portaki release --dry-run  # everything up to the push
```

`release` asks the Portaki registry for the right to push this version
(`POST /registry/v1/publications/push-token`): short-lived, limited to this module and version, for
the repository `modules/<module-id>` of the OCI host the registry names. No `docker login`, no
GitHub token, no registry to choose. It then pushes the artifact, attests the pushed digest as its
author (`cosign attest`, your GitHub identity — `--no-sign` skips it, and the version then runs in
the sandbox only), and announces `oci://<host>/modules/<module-id>@<digest>` to the registry.
Replaying a version that is already published is not an error: publications are immutable.

From CI, two jobs: `portaki ci build` runs the module's code (tests, `build.rs`) without any
right; `portaki ci release` pushes what it built, signs it with the workflow's identity
(SLSA provenance and `cargo audit`), and announces it — running nothing of the module, not even
cargo. [`portaki-release-action@v2`](https://github.com/PortakiApp/portaki-release-action) wires
both.

The artifact carries the catalog written by `portaki build` from the code (merged over a
`portaki.module.json` when the module still keeps one) and the SDK layer
(plus optional migrations / operations / i18n / wasm):

- `application/vnd.portaki.manifest+json` — host catalog (`publish-manifest.json`)
- `application/vnd.portaki.sdk.manifest+json` — SDK emissions (`manifest.json`)
- `application/vnd.portaki.migrations+json` — `migrations.bundle.json` (when `db/migrations/` exists)
- `application/vnd.portaki.operations+json` — `operations.bundle.json` v2 schema from `#[entity]` (typed-repo upsert)

## Related repositories

| Repository | Role |
|------------|------|
| [portaki-modules](https://github.com/PortakiApp/portaki-modules) | Official Wasm modules monorepo |
| [portaki.app](https://portaki.app) | Product site |

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md) and the [Code of Conduct](./CODE_OF_CONDUCT.md).

Security issues: [SECURITY.md](./SECURITY.md) — do not open a public issue.

## License

[Apache-2.0](./LICENSE) · Copyright 2026 Syntax Labs
