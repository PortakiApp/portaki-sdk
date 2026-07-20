# GitHub Actions CI

- Several jobs per quality workflow — never one monolith.
- **One job = one subject** (`fmt`, `clippy`, `test`, …).
- Final **`quality` gate** (`if: always()`) aggregates job results.
- Artifacts (when used): `retention-days: 1` (GitHub minimum).

Local Cursor mirror (gitignored): `.cursor/rules/github-actions-ci.mdc`.
