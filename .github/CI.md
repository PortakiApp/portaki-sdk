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

## Dépendances — un seul gestionnaire

**Renovate écrit les PR. Dependabot ne les écrit plus.** Ce dépôt est celui où la double
installation a coûté quelque chose : le 7 septembre Renovate montait `syn` en patch (PR #46,
groupée, conforme) ; le 9, Dependabot le montait de `2.0.119` à `3.0.5` — une majeure (PR #74),
mergée. Même chose pour `getrandom 0.3→0.4` et `base64 0.22→0.23`.

La règle « Majors Rust — review manuelle » vit dans `renovate.json`. Dependabot ne la lit pas :
il ne l'a pas contournée, il ne l'a jamais vue. C'est la raison de fond pour n'en garder qu'un —
une politique qu'un second robot ignore n'est pas une politique.

La règle reste **une étiquette `breaking` sur une PR**, pas un blocage : c'est bien une revue
manuelle, puisqu'un humain merge. Ce qui manquait n'était pas le verrou, c'était l'unicité.

Ce qui reste de Dependabot, et qui n'a rien à voir avec ce fichier : les **alertes de
vulnérabilité** et le **graphe de dépendances**, activés côté GitHub, plus le **secret scanning**
avec sa protection au push — ce dépôt est public, une clé poussée y est indexée dans la seconde.
Les *security updates* automatiques restent éteints : `vulnerabilityAlerts` de Renovate est déjà
réglé sur `at any time`.

Local Cursor mirror (gitignored): `.cursor/rules/github-actions-ci.mdc`.
