# Releases — portaki-sdk

## Flow

1. Land Conventional Commits on `main` via rebase PR (`feat:`, `fix:`, …).
2. **Release please** opens / updates a draft PR `chore: release X.Y.Z` with `CHANGELOG.md` + Cargo workspace version bumps.
3. Review, merge (rebase) → git tag `vX.Y.Z` + GitHub Release.
4. Tag push runs **Publish crates.io** → `cargo ws publish` for the workspace (OIDC).
5. Downstream modules can depend on crates.io versions, or keep `git` + `branch` / `tag` until they switch.

## Version source of truth

Workspace version lives in root `Cargo.toml`:

- `workspace.package.version`
- `workspace.dependencies.portaki-*.version` (path crates)

Manifest: [`.release-please-manifest.json`](../.release-please-manifest.json)  
Config: [`release-please-config.json`](../release-please-config.json)

## crates.io

Published crates (same semver):

| Crate | Role |
|-------|------|
| `portaki-sdk-macros` | Proc-macros |
| `portaki-sdk` | Authoring SDK |
| `portaki-connectors` | Built-in connectors |
| `portaki-test-utils` | Test harness |
| `portaki-cli` | Binary `portaki` (`cargo install portaki-cli`) |

Workflow: [`.github/workflows/publish-crates.yml`](../.github/workflows/publish-crates.yml)  
Tools: `rust-lang/crates-io-auth-action` + [`cargo-workspaces`](https://github.com/pksunkara/cargo-workspaces) (`cargo ws publish`).

### First publish (bootstrap)

Trusted Publishing only works **after** each crate exists on crates.io.

1. Create a [crates.io](https://crates.io) account linked to GitHub.
2. Create an API token (crates.io → Account → API Tokens) with publish scope.
3. From a clean checkout of the release commit:

```bash
export CARGO_REGISTRY_TOKEN=…   # one-shot; do not commit
cargo publish -p portaki-sdk-macros
# wait ~15s for index
cargo publish -p portaki-sdk
cargo publish -p portaki-connectors
cargo publish -p portaki-test-utils
cargo publish -p portaki-cli
```

Order matters: `portaki-test-utils` depends on `portaki-sdk`, so the SDK must
publish first. Do **not** add `portaki-test-utils` as a `[dev-dependency]` of
`portaki-sdk` — that creates a publish-time cycle (`cargo publish` resolves
dev-deps from crates.io).

4. For **each** crate → Settings → Trusted Publishing → Add:
   - Repository owner: `PortakiApp`
   - Repository name: `portaki-sdk`
   - Workflow filename: `publish-crates.yml`
   - Environment: `crates-io`
5. Later tags: OIDC only — no long-lived token in GitHub secrets.

The `crates-io` environment only deploys `v*` tags, and the job refuses any other ref: nothing
reaches crates.io from a branch.

### Resume a partial publish

If a tag run published some crates then failed (e.g. index lag):

1. Actions → the failed run → **Re-run failed jobs**, or **Publish crates.io** → **Run workflow**
   on the tag `vX.Y.Z` (not on `main`: the job refuses a branch).
2. Already-published crate versions are skipped by `cargo ws publish`.

A failure that needs a code change is fixed on `main` and released as the next patch version.

## Prebuilt CLI binaries

Workflow: [`.github/workflows/publish-cli-binaries.yml`](../.github/workflows/publish-cli-binaries.yml),
on the same `v*` tag push. It builds `portaki` in release mode and attaches to the GitHub Release
`vX.Y.Z`:

| Asset | Content |
|-------|---------|
| `portaki-X.Y.Z-x86_64-unknown-linux-gnu.tar.gz` | `portaki` (built on `ubuntu-latest`) |
| `portaki-X.Y.Z-aarch64-apple-darwin.tar.gz` | `portaki` (built on `macos-latest`) |
| `<archive>.sha256` | `sha256sum` line for that archive |
| `SHA256SUMS` | all archives |

`PortakiApp/portaki-release-action/install` downloads the archive matching the runner, checks its
`.sha256`, and falls back to `cargo install portaki-cli` when the release has no such asset
(versions before this workflow) or the runner has no supported target.

Trust: only this workflow writes these assets. It compiles the SDK workspace at the tag and
nothing else — no module code, no Rust cache — and the only job holding `contents: write`
(`release`) compiles nothing: it checksums the build artifacts and runs `gh release upload`.
No secret besides `GITHUB_TOKEN`.

The build runs `cargo update --workspace` before `cargo build --locked`: release-please bumps the
workspace version in `Cargo.toml` but not in `Cargo.lock`. Only workspace members move; every
third-party crate stays at its locked version.

Replay after a failure: **Re-run jobs** on the tag run (`--clobber` replaces partial uploads).

## Secrets (GitHub)

| Secret | Usage |
|--------|--------|
| `CI_APP_ID` / `CI_APP_PRIVATE_KEY` | release-please (draft PR + tag + GitHub Release) |

App permissions: **Contents** R/W, **Pull requests** R/W, **Metadata** R.

Branch / tag rulesets: org Settings → Rules (not stored in this repo).
