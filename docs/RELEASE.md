# Releases — portaki-sdk

## Flow

1. Land Conventional Commits on `main` via rebase PR (`feat:`, `fix:`, …).
2. **Release please** opens / updates a draft PR `chore: release X.Y.Z` with `CHANGELOG.md` + Cargo workspace version bumps.
3. Review, merge (rebase) → git tag `vX.Y.Z` + GitHub Release.
4. Tag push runs **Publish crates.io** → `cargo publish` for the workspace (OIDC).
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
Actions: `rust-lang/crates-io-auth-action` + `katyo/publish-crates`.

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

4. For **each** crate → Settings → Trusted Publishing → Add:
   - Repository owner: `PortakiApp`
   - Repository name: `portaki-sdk`
   - Workflow filename: `publish-crates.yml`
5. Later tags: OIDC only — no long-lived token in GitHub secrets.

## Secrets (GitHub)

| Secret | Usage |
|--------|--------|
| `CI_APP_ID` / `CI_APP_PRIVATE_KEY` | release-please (draft PR + tag + GitHub Release) |

App permissions: **Contents** R/W, **Pull requests** R/W, **Metadata** R.

Branch / tag rulesets: org Settings → Rules (not stored in this repo).
