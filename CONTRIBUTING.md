# Contributing to portaki-sdk

Thanks for helping improve the Portaki module SDK. This document covers the basics for a clean pull request.

## Development setup

```bash
git clone https://github.com/PortakiApp/portaki-sdk.git
cd portaki-sdk
rustup target add wasm32-unknown-unknown
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Pull requests

1. Open a PR against **`main`**. Do not push straight to `main`.
2. Keep changes focused — one concern per PR when possible.
3. Update docs/tests when behavior changes.
4. Use [Conventional Commits](https://www.conventionalcommits.org/) in English, e.g. `feat(cli): …`, `fix(sdk): …`.

### Quality gates (must pass)

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

CI runs the same checks on every PR.

## Workspace layout

| Path | Notes |
|------|--------|
| `crates/portaki-sdk` | Public authoring APIs — avoid breaking changes without a clear migration |
| `crates/portaki-sdk-macros` | Compile-time emissions consumed by `portaki build` / `portaki lint` |
| `crates/portaki-cli` | Binary UX — keep `--help` accurate |
| `crates/portaki-connectors` | External connector clients used by modules |
| `crates/portaki-test-utils` | Prefer these helpers over ad-hoc mocks in new tests |
| `templates/` | Scaffolding for `portaki init` |

## Coding guidelines

- Prefer explicit types and short functions over clever abstractions.
- No emojis in user-facing CLI output or docs unless already established.
- Public API docs (`///`) on exported items; skip restating the obvious.
- Wasm-facing code must stay free of `wasm-bindgen` imports in release artifacts.

## Reporting security issues

Do **not** open a public issue for vulnerabilities. Follow [SECURITY.md](./SECURITY.md).

## License

By contributing, you agree that your contributions are licensed under the [Apache License 2.0](./LICENSE).
