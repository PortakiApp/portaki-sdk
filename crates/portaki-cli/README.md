# portaki-cli

Command-line toolchain for Portaki Wasm modules — binary name **`portaki`**.

| Command | Purpose |
|---------|---------|
| `portaki init` | Scaffold a module from a template |
| `portaki build` | Compile `wasm32`, merge macro emissions → `manifest.json` |
| `portaki lint` | Validate manifest, i18n keys, capability ids |
| `portaki test` | Run the module crate’s `cargo test` |
| `portaki publish` | Push the OCI artifact to a registry (e.g. GHCR) |
| `portaki catalog` | Dump the SDUI catalog specification |
| `portaki inspect` | Inspect a published OCI artifact |
| `portaki docs` / `dev` | Docs helper / local mock gateway (stubs evolve with SDK) |

## Install

```bash
cargo install portaki-cli
# or from git tip:
cargo install --git https://github.com/PortakiApp/portaki-sdk --locked portaki-cli
```

Requires the `wasm32-unknown-unknown` target for builds:

```bash
rustup target add wasm32-unknown-unknown
```

## Typical workflow

```bash
cd modules/weather
portaki build --release
portaki lint
PORTAKI_PUBLISH_VERSION=0.3.5 portaki publish --registry ghcr.io/portakiapp
```

Companion crates: [`portaki-sdk`](https://crates.io/crates/portaki-sdk), [`portaki-sdk-macros`](https://crates.io/crates/portaki-sdk-macros).

## License

Apache-2.0
