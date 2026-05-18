# Guide d’utilisation — Portaki SDK

Monorepo [`portaki-sdk`](https://github.com/PortakiApp/portaki-sdk) : **`@portaki/sdk`**, **`@portaki/cli`**, **`@portaki/sdk-test-support`**.

---

## 1. `@portaki/sdk` — module invité et backend TypeScript

### Rôle

- **UI invité** : React (`definePortakiModule` ou `defineModule` avec segment `render`).
- **Données & gateway** : schéma DSL (`moduleSchema`, `table`, …), handlers `queries` / `commands` avec `ctx.db`.
- **Manifeste** : validation via `moduleSchema` aligné sur `schema/module.v1.json`.

### API principales

| Symbole | Rôle |
|---------|------|
| `definePortakiModule(def)` | Module invité UI seul (export default npm). |
| `defineModule(input)` | Module complet : schéma + handlers + UI. |
| `moduleSchema`, `table`, … | DSL de schéma Postgres (migrations générées par le CLI). |
| Hooks livret | `useTracking`, `useGuestContext`, … (`@portaki/sdk` ou `@portaki/sdk/runtime`). |

**Peer** : `react >= 18`.

Voir **[module-authoring.md](./module-authoring.md)** et l’exemple **`examples/rules/`**.

---

## 2. `@portaki/cli`

```bash
pnpm add -D @portaki/cli
pnpm exec portaki build
```

Génère `.portaki/` (manifeste backend, bundle migrations, artefact Wasm cible).

---

## 3. `@portaki/sdk-test-support` (dev)

Helpers Vitest pour invoquer les handlers gateway en tests unitaires.

---

## 4. Publication

**[deployment.md](./deployment.md)** — CI **Release** sur `main`, tags GitHub **`v*`** alignés sur `packages/sdk/package.json`.
