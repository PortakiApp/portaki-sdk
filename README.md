<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/PortakiApp/portaki-sdk/develop/docs/assets/portaki-wordmark-light.svg">
    <img src="https://portaki.app/portaki-wordmark.svg" width="177" height="48" alt="Portaki" />
  </picture>
</p>

<h1 align="center">Portaki SDK</h1>

<p align="center">
  <strong>Monorepo officiel</strong> — <code>@portaki/sdk</code>, <code>@portaki/cli</code>, tests<br/>
  <sub>pnpm workspace · modules catalogue dans <a href="https://github.com/PortakiApp/portaki-modules">portaki-modules</a></sub>
</p>

---

## Structure

```
portaki-sdk/
  packages/
    sdk/                 @portaki/sdk — defineModule, UI, schema, hooks
    cli/                 @portaki/cli — portaki build
    sdk-test-support/    @portaki/sdk-test-support (dev)
  examples/rules/
  schema/module.v1.json
```

## Exports `@portaki/sdk`

| Subpath | Usage |
|---------|--------|
| `.` | `defineModule`, `definePortakiModule`, schema DSL, UI, hooks |
| `./runtime` | hooks / slots (import optionnel) |
| `./server` | HMAC, Route Handlers (app hôte Next) |
| `./build` | réservé CLI (enregistrement d’opérations) |

## Module author deps

```json
{
  "dependencies": { "@portaki/sdk": "^2.0.0" },
  "devDependencies": {
    "@portaki/cli": "^0.1.0",
    "@portaki/sdk-test-support": "^2.0.0"
  }
}
```

Voir **[docs/module-authoring.md](docs/module-authoring.md)** · **[docs/getting-started.md](docs/getting-started.md)** · **[docs/deployment.md](docs/deployment.md)**.

## Documentation (guides + API)

| Commande | Sortie |
|----------|--------|
| `pnpm docs:dev` | Site VitePress local (`docs-site/`) |
| `pnpm docs:build` | Build statique + TypeDoc API → prêt Vercel |
| `pnpm docs:api` | Référence TypeScript → `docs-site/public/api/` |
| `pnpm docs:doxygen` | HTML Doxygen → `build/doxygen/html/` (optionnel) |

Conventions commentaires source : **[docs/code-documentation.md](docs/code-documentation.md)**.  
Hébergement (GitBook, VitePress, Mintlify, …) : **[docs/hosting.md](docs/hosting.md)**.

Référence exemple : **`examples/rules/`** (module gateway complet).

## Développement

```bash
pnpm install && pnpm test
pnpm test:coverage   # SDK + CLI + test-support (seuils Vitest v8)
```

Tests d’intégration CLI : `examples/rules` → `portaki build`.

## CI / release

| Workflow | Rôle |
|----------|------|
| **CI** (`ci.yml`) | `verify` — build + coverage sur PR / `main` / `develop` |
| **Release** (`release.yml`) | Après CI verte sur `main` : npm + release GitHub `v{semver}` |
