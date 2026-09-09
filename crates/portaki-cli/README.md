<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://portaki.app/logo-dark.svg">
    <img src="https://portaki.app/logo-light.svg" width="177" height="48" alt="Portaki">
  </picture>
</p>

<h1 align="center">portaki-cli</h1>

<p align="center">
  <strong>Command-line toolchain for Portaki Wasm modules</strong><br>
  Binary name <code>portaki</code> — init, build, lint, test, and OCI publish.
</p>

<p align="center">
  <a href="https://crates.io/crates/portaki-cli"><img src="https://img.shields.io/crates/v/portaki-cli.svg" alt="crates.io"></a>
  <a href="https://docs.rs/portaki-cli"><img src="https://img.shields.io/docsrs/portaki-cli" alt="docs.rs"></a>
  <a href="https://github.com/PortakiApp/portaki-sdk/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-blue.svg" alt="License Apache-2.0"></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-1.75+-dea584?logo=rust&logoColor=white" alt="Rust 1.75+"></a>
  <a href="https://extism.org/"><img src="https://img.shields.io/badge/Extism-Wasm-7C3AED" alt="Extism"></a>
  <a href="https://portaki.app"><img src="https://img.shields.io/badge/site-portaki.app-f59e0b" alt="portaki.app"></a>
</p>

<p align="center">
  <a href="#install">Install</a> ·
  <a href="#commands">Commands</a> ·
  <a href="#typical-workflow">Workflow</a> ·
  <a href="#related-crates">Crates</a> ·
  <a href="#license">License</a>
</p>

---

Authors write modules against [`portaki-sdk`](https://crates.io/crates/portaki-sdk). At build time this binary compiles to `wasm32`, merges proc-macro emissions from `OUT_DIR/portaki-emissions/`, and packages OCI layers for registries such as GHCR.

## Install

```bash
cargo install portaki-cli
# or tip of main:
cargo install --git https://github.com/PortakiApp/portaki-sdk --locked portaki-cli
```

Requires the Wasm target:

```bash
rustup target add wasm32-unknown-unknown
```

## Commands

| Command | Contract |
|---------|----------|
| `portaki init` | Scaffold a module from a template |
| `portaki build` | Compile Wasm + merge emissions → `manifest.json`, tamponne la version SDK liée |
| `portaki lint` | Validate capabilities, connectors, i18n keys |
| `portaki test` | Forward to `cargo test` in the module crate |
| `portaki publish` | Push the OCI artifact, then announce it to the registry |
| `portaki catalog` | Dump the SDUI primitive catalog |
| `portaki inspect` | Inspect a published OCI artifact |
| `portaki docs` / `dev` | Docs helper / local mock gateway (evolves with the SDK) |

## Output

Every command writes the same way: a line under the title saying what it actually does, one step
per line, a spinner while it runs, the elapsed time once it is done, and a `next` block naming
what to run afterwards and what each one gives you. The tools the CLI drives (`cargo build`,
`cargo test`) stay quiet unless they fail — then their whole output surfaces, because that is
what you were looking for.

The explanatory lines earn their place: `dev` does not start a local gateway, `publish` does not
just push, and `init` leaves a tree whose halves (`ids.rs` and `i18n/`) only make sense together.
Saying so costs a line each.

| Flag | Effect |
|------|--------|
| `--no-color` | Plain text, no colour and no spinners |
| `-v`, `--verbose` | Stream the raw output of the tools the CLI drives |

Colour and animation turn themselves off when the output is not a terminal, and `NO_COLOR` is
honoured. `portaki catalog` and `portaki inspect` write nothing but their JSON to stdout, so
they stay pipeable into `jq`.

`portaki login` opens the browser on the verification URL — pre-filled with the code when the
platform returns one, so there is nothing left to paste. The code is printed either way; use
`--no-browser` over SSH or on a headless box.

## Typical workflow

```bash
cd modules/weather
portaki build --release
portaki lint
PORTAKI_PUBLISH_VERSION=0.3.5 portaki publish --registry ghcr.io/portakiapp
```

Image name: `ghcr.io/portakiapp/portaki-modules-<module-id>:<semver>`.

`publish` announces the version to the registry after the push (needs `portaki login`).
`--no-announce` skips it — the artifact then belongs to no catalogue. `--announce-only` announces
a version already on GHCR without pushing anything, which is how an existing catalogue is adopted.

### Publishing from CI

No publication secret to store. In a GitHub Actions job with `id-token: write`, the CLI asks
GitHub for the job's OIDC token and exchanges it at the registry for a single-use publication
credential:

```yaml
permissions:
  contents: read
  packages: write
  id-token: write     # sans quoi il n'y a pas de jeton à échanger

jobs:
  publish:
    environment: release   # exigé par la liaison pour le canal stable
    steps:
      - run: portaki publish --registry ghcr.io/portakiapp
```

Link the module to its repository from the dashboard first: the registry authorises on the
repository *id* recorded there, and checks the workflow file, the triggering event and the
runner. The token says where it comes from; the link says what it may publish.

`PORTAKI_DEV_TOKEN` still wins when it is set — an explicit choice beats a mechanism that turns
itself on.

Official modules: [`portaki-modules`](https://github.com/PortakiApp/portaki-modules).

## Related crates

| Crate | Role |
|-------|------|
| [`portaki-sdk`](https://crates.io/crates/portaki-sdk) | Host APIs + SDUI |
| [`portaki-sdk-macros`](https://crates.io/crates/portaki-sdk-macros) | Manifest emissions |
| [`portaki-connectors`](https://crates.io/crates/portaki-connectors) | Typed connector ops |
| [`portaki-test-utils`](https://crates.io/crates/portaki-test-utils) | Mock host for tests |

## License

[Apache-2.0](https://github.com/PortakiApp/portaki-sdk/blob/main/LICENSE) · Copyright 2026 Syntax Labs
