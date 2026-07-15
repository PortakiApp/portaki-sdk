# Branch policy — portaki-sdk

Repo: **PortakiApp/portaki-sdk** · default branch: **`main`**

Org **GitHub Team** — repo **rulesets** (`Settings → Rules`, CI GitHub App as bypass).

Release details: **[docs/RELEASE.md](../docs/RELEASE.md)**

## Flow

```
feat/fix/* ──PR──► main ──► CI (quality)
                    │
                    └──► release-please PR ──merge──► tag vX.Y.Z + GitHub Release
```

| Branch | Role |
|--------|------|
| `main` | Integration |
| `feat/*`, `fix/*` | Work |
| Tag `v*` | SDK release |

Consumers (e.g. `portaki-modules`) resolve crates via `git` + `branch = "main"` or a semver tag when published.

## Merge settings

| Setting | Value |
|---------|--------|
| **Rebase and merge** | **Only allowed method** |
| Create a merge commit | Off |
| Squash | Off |
| Delete head branch on merge | **Yes** |

`main` stays linear — Conventional Commits (`feat:`, `fix:`…) for release-please.

## Rulesets

Three active rulesets (`configure-repo-rulesets.mjs`):

| Ruleset | Target | Checks |
|---------|--------|--------|
| `portaki-sdk: branch integrity` | `main` | no force-push / delete |
| `portaki-sdk: main integration` | `main` | PR + `quality` (strict) |
| `portaki-sdk: release tags` | `refs/tags/v*` | creation reserved to CI bypass |

**Bypass**: org CI GitHub App — tags `v*` (release-please).

> **Why not `GITHUB_TOKEN`?** Events from `GITHUB_TOKEN` do not trigger other workflows. release-please uses the **CI App** token so tag + GitHub Release can fan out.

```bash
CI_APP_ID=… GITHUB_TOKEN=ghp_… node .github/scripts/configure-repo-rulesets.mjs
```

## Workflows

| Workflow | Trigger |
|----------|---------|
| **ci** | PR + push `main` → `rust` + aggregator **`quality`** |
| **Release please** | push `main` (CI App token) |

## Secrets

| Secret | Usage |
|--------|--------|
| `CI_APP_ID` / `CI_APP_PRIVATE_KEY` | release-please (PR + tag + release) + ruleset bypass for `v*` |

App permissions: **Contents** R/W, **Pull requests** R/W, **Metadata** R.
