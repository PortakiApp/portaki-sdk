<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://portaki.app/logo-dark.svg">
    <img src="https://portaki.app/logo-light.svg" width="177" height="48" alt="Portaki">
  </picture>
</p>

<h1 align="center">portaki-sdk</h1>

<p align="center">
  <strong>Rust SDK, CLI, connectors, and test utilities for Portaki Wasm modules</strong><br>
  Write, test, and publish the modules that run in Portaki's digital welcome booklet.
</p>

<p align="center">
  <a href="https://github.com/PortakiApp/portaki-sdk/actions/workflows/ci.yml"><img src="https://github.com/PortakiApp/portaki-sdk/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://crates.io/crates/portaki-sdk"><img src="https://img.shields.io/crates/v/portaki-sdk.svg" alt="crates.io"></a>
  <a href="https://docs.rs/portaki-sdk"><img src="https://img.shields.io/docsrs/portaki-sdk" alt="docs.rs"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-blue.svg" alt="License Apache-2.0"></a>
</p>

---

[Portaki](https://portaki.app) is a digital welcome booklet for short-term rental hosts. Features
of the booklet (weather, nearby places, check-in, house rules, …) are **modules**: Rust crates
compiled to `wasm32-unknown-unknown` and run by the platform as [Extism](https://extism.org/)
plugins. Modules never do raw I/O; they call host functions and return server-driven UI (SDUI)
that the host dashboard and the guest booklet render.

This workspace is everything a module author needs: the host APIs, the proc-macros that describe a
module at compile time, typed connectors, an in-process mock host for tests, and the `portaki` CLI.
The official modules live in [`portaki-modules`](https://github.com/PortakiApp/portaki-modules) and
are good real-world examples.

## Workspace crates

All five crates are published on crates.io with the same version (currently **10.17.0**).

| Crate | Purpose |
|-------|---------|
| [`portaki-sdk`](./crates/portaki-sdk) · [docs.rs](https://docs.rs/portaki-sdk) | Host function wrappers, SDUI catalog, capability constants, module macros (re-exported) |
| [`portaki-sdk-macros`](./crates/portaki-sdk-macros) · [docs.rs](https://docs.rs/portaki-sdk-macros) | Proc-macros that emit the manifest at compile time, read by `portaki build` |
| [`portaki-connectors`](./crates/portaki-connectors) · [docs.rs](https://docs.rs/portaki-connectors) | Typed clients for platform-catalogued connectors: OpenWeather, OpenAgenda, Google Places, Mapbox, OSM Nominatim, Nuki |
| [`portaki-test-utils`](./crates/portaki-test-utils) · [docs.rs](https://docs.rs/portaki-test-utils) | `MockContext`, in-memory host functions, SDUI assertions, conformance checks |
| [`portaki-cli`](./crates/portaki-cli) | The `portaki` binary |

### `portaki-sdk` features

Host APIs are opt-in. Each feature also declares the matching permission in the module manifest.

| Feature | Enables |
|---------|---------|
| `kv` | Key-value storage |
| `repo` | Typed repositories over the module's entities |
| `email` | Sending e-mail |
| `events` | Emitting domain events (`host::events`) |
| `platform` | Module status and config completeness (`host::module`); on in the default template |
| `guest-files` | Declares the `guest:files` permission (guest uploads) |
| `host-files` | `host::files::read`, files the host attached to the module |
| `stay-guest-contact` | Declares access to the guest's e-mail and phone on the stay context |
| `unstable` | Declared resources (`#[item]`, `#[collection]`); API may change in any release |

`portaki add` declares permissions, connectors, and languages for you, so you rarely edit these by
hand. `portaki-connectors` has a single `mock` feature, turned on by `portaki-test-utils` only.

## Requirements

- Rust stable (CI builds on the latest stable toolchain)
- The Wasm target:

```bash
rustup target add wasm32-unknown-unknown
```

## Install the CLI

From crates.io:

```bash
cargo install portaki-cli --locked
```

Or download a prebuilt binary (Linux x86_64, macOS arm64, with SHA-256 checksums) from the
[latest release](https://github.com/PortakiApp/portaki-sdk/releases/latest).

`portaki doctor` checks your machine: session, versions, toolchain, cosign, registry.

## Quick start

```bash
portaki init my-module        # scaffold from the default template (or --template empty)
cd my-module
portaki build                 # wasm32 artifact, manifest, operations, i18n bundle in target/portaki/
portaki test                  # cargo test, with the mock host and the conformance battery
portaki check                 # the gate `portaki release` applies: fmt, clippy, tests, wasm build, manifest, texts
```

The template ships a `listing.json` (the public catalogue entry) that the conformance tests refuse
until you fill it in, or delete it to write the listing in the dashboard.

To run your module against a real platform, sign in and use the hosted sandbox:

```bash
portaki login                 # device grant: approve the code in your browser
portaki dev                   # build, push to the sandbox, show what the run did
portaki preview               # render a surface of the sandbox build
portaki logs                  # follow the module's logs, live
```

Run `portaki --help` for the full list (`status`, `run`, `scenarios`, `connectors`, `upgrade`,
`reports`, …) and `portaki <command> --help` for each one.

Credentials are stored in `~/.config/portaki/credentials.json` (mode `0600`), not in the system
keychain; see [docs/cli-keychain-macos.md](docs/cli-keychain-macos.md).

## Release a module

```bash
portaki release --dry-run     # tests, build, and package, nothing pushed
portaki release               # tests, build, push to Portaki's registry, sign, announce
portaki release status        # where a published version stands
```

`release` asks the Portaki registry for a short-lived push right limited to this module and
version: no `docker login`, no registry to configure. It pushes the OCI artifact, attests its
digest with `cosign` under your GitHub identity, and announces it. `--no-sign` skips the signature;
the version then only runs in the sandbox. Published versions are immutable.

From CI, split it in two jobs: `portaki ci build` runs the module's code (tests, `build.rs`) with
no publishing rights, and `portaki ci release` pushes, signs with the workflow's identity, and
announces, without running any module code.
[`portaki-release-action`](https://github.com/PortakiApp/portaki-release-action) wires both; the
default template includes a ready-made `.github/workflows/release.yml`.

The artifact carries the catalog written by `portaki build`
(`application/vnd.portaki.manifest+json`), the SDK emissions
(`application/vnd.portaki.sdk.manifest+json`), the wasm, and, when present, migrations
(`db/migrations/`) and the typed-repo operations bundle.

## Documentation

| Topic | |
|-------|---|
| Module layout | [docs/module-layout.md](docs/module-layout.md) |
| Connectors and credentials (pool / BYOK) | [docs/connectors-and-credentials.md](docs/connectors-and-credentials.md) |
| Host surfaces and contracts | [docs/host-surfaces-and-contracts.md](docs/host-surfaces-and-contracts.md) |
| Guest operations | [docs/guest-operations.md](docs/guest-operations.md) |
| Typed ids | [docs/typed-ids.md](docs/typed-ids.md) |
| SDUI guest inventory | [docs/sdui-guest-inventory.md](docs/sdui-guest-inventory.md) |
| How the SDK itself is released | [docs/RELEASE.md](docs/RELEASE.md) |
| API reference | [docs.rs/portaki-sdk](https://docs.rs/portaki-sdk) |

## Repository layout

| Path | Contents |
|------|----------|
| `crates/` | The five published crates |
| `crates/portaki-cli/templates/` | `portaki init` templates (`default-module`, `empty-module`) |
| `contracts/` | JSON contracts shared with the platform (host ops, SDUI types, deprecations, scenarios) |
| `schema/` | JSON Schemas for `module.v1` and `listing.v1` |
| `docs/` | Guides linked above |
| `scripts/` | Contract bundling, mock boundary check, local CLI install (macOS signing) |

## Develop on the SDK

```bash
git clone https://github.com/PortakiApp/portaki-sdk.git
cd portaki-sdk
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p portaki-cli -- --help
```

Releases are cut by release-please from Conventional Commits on `main`; a tag publishes the crates
to crates.io and the CLI binaries to the GitHub release.

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md) and the [Code of Conduct](./CODE_OF_CONDUCT.md).
Security issues: follow [SECURITY.md](./SECURITY.md); do not open a public issue.

## License

[Apache-2.0](./LICENSE) · Copyright 2026 Syntax Labs
