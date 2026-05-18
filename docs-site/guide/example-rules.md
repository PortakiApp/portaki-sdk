# Exemple — module `rules`

Référence officielle dans ce dépôt : [`examples/rules/`](https://github.com/PortakiApp/portaki-sdk/tree/main/examples/rules).

## Rôle

Module **gateway + schéma** minimal :

- Table `t_e_module_rules_content` (DSL `moduleSchema`)
- Query `rules.content` (lecture invité)
- Command `rules.content.save` (écriture hôte)
- UI invité `RulesGuestView`

## Build

```bash
cd examples/rules
pnpm exec portaki build --entry src/portaki.module.ts --manifest portaki.module.json
```

Artefacts dans `.portaki/` (gitignored) :

- `migrations.bundle.json`
- `operations.bundle.json`

## Tests

Les tests du package `examples/rules` utilisent `@portaki/sdk-test-support` pour valider le manifeste et le rendu.
