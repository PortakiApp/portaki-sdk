# Releases — portaki-sdk

## Flow

1. Land Conventional Commits on `main` via rebase PR (`feat:`, `fix:`, …).
2. **Release please** opens / updates a draft PR `chore: release X.Y.Z` with `CHANGELOG.md` + Cargo workspace version bumps.
3. Review, merge (rebase) → git tag `vX.Y.Z` + GitHub Release.
4. Downstream modules (`portaki-modules`) pick up `main` / the tag via `cargo update`.

## Version source of truth

Workspace version lives in root `Cargo.toml`:

- `workspace.package.version`
- `workspace.dependencies.portaki-*.version` (path crates)

Manifest: [`.release-please-manifest.json`](../.release-please-manifest.json)  
Config: [`release-please-config.json`](../release-please-config.json)

## Secrets

Repo secrets **`CI_APP_ID`** and **`CI_APP_PRIVATE_KEY`** must match the org CI GitHub App (same pattern as `portaki-web`).

## Rulesets

See [BRANCH_POLICY.md](../.github/BRANCH_POLICY.md). Apply:

```bash
CI_APP_ID=… GITHUB_TOKEN=ghp_… node .github/scripts/configure-repo-rulesets.mjs
```
