# GitHub Actions CI

## Flow

1. **`rust`** — fmt + clippy + test + doc on one runner.
2. **`quality`** gate.

## Minutes

GitHub bills **job-minutes**. Splitting fmt/clippy/test into three jobs ≈ 3× billed time for the same toolchain setup.

- Prefer one rust job unless subjects truly need different runners/matrices.
- `concurrency` + `cancel-in-progress` on PRs.
- Post-merge `push` to `main` skips rust when commits already ran CI on a merged PR.
- `paths-ignore` for docs-only changes.
- Clean trybuild nests (`target/tests`) before rust-cache save.

Local Cursor mirror (gitignored): `.cursor/rules/github-actions-ci.mdc`.
