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

## Dependencies — one bot only

**Renovate writes the PRs. Dependabot no longer does.** This is the repo where running both cost
us something: on 7 September Renovate bumped `syn` by a patch (PR #46, grouped, in line with the
rules); on the 9th, Dependabot bumped it from `2.0.119` to `3.0.5` — a major (PR #74), and it got
merged. Same story for `getrandom 0.3→0.4` and `base64 0.22→0.23`.

The "Majors Rust — review manuelle" rule lives in `renovate.json`. Dependabot does not read it: it
did not work around the rule, it never saw it. That is the underlying reason to keep only one bot
— a policy a second robot ignores is not a policy.

### Automerge

Patch, `pin` and `digest` merge on their own once `quality` is green. Majors never merge on their
own — and **neither do `0.x` minors**: semver does not apply before `1.0`, a minor there breaks
like a major anywhere else, and half the Rust ecosystem is on `0.x`.

A bump waits **three days** after publication. Without that delay there is a direct path from a
maintainer's crates.io account to what we ship. Security fixes do not wait.

`extism-pdk` is excluded from automerge whatever the bump type: it is the boundary with the host
runtime, and a bump that is not matched on the other side breaks modules at run time, not at
compile time.

The configuration lives in `PortakiApp/renovate-config`.

The rule is still **a `breaking` label on a PR**, not a block: it really is a manual review, since
a human does the merge. What was missing was not the lock, it was having a single bot.

What is left of Dependabot, and has nothing to do with this file: the **vulnerability alerts** and
the **dependency graph**, enabled on the GitHub side, plus **secret scanning** with its push
protection — this repo is public, a key pushed here is indexed within the second. Automatic
*security updates* stay off: Renovate's `vulnerabilityAlerts` is already set to `at any time`.

Local Cursor mirror (gitignored): `.cursor/rules/github-actions-ci.mdc`.
