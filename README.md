<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/PortakiApp/portaki-sdk/develop/docs/assets/portaki-wordmark-light.svg">
    <img src="https://portaki.app/portaki-wordmark.svg" width="177" height="48" alt="Portaki" />
  </picture>
</p>

<h1 align="center">Portaki SDK</h1>

<p align="center">
  <strong>Monorepo officiel</strong> — <code>@portaki/sdk</code>, <code>@portaki/cli</code>, tests ; Java legacy pour backends JAR<br/>
  <sub>pnpm workspace · modules catalogue dans <a href="https://github.com/PortakiApp/portaki-modules">portaki-modules</a></sub>
</p>

---

## Structure

```
portaki-sdk/
  packages/
    sdk/                 @portaki/sdk — defineModule, UI, schema, hooks
    cli/                 @portaki/cli
    sdk-test-support/    @portaki/sdk-test-support (dev)
  examples/rules/
  schema/module.v1.json
  legacy/java/           Maven (backends JAR existants)
```

## Exports `@portaki/sdk`

| Subpath | Usage |
|---------|--------|
| `.` | `defineModule`, schema DSL, UI, hooks, handler types |
| `./runtime` | same hooks/slots (optional split import) |
| `./server` | HMAC, Route Handlers (host Next app) |
| `./build` | CLI-only (operation recording) |

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

Voir **[docs/module-authoring.md](docs/module-authoring.md)**.

## Développement

```bash
pnpm install && pnpm test
pnpm test:coverage   # SDK + CLI + test-support (seuils Vitest v8)
```

Tests d'intégration CLI : `examples/rules` → `run-build` + binaire `portaki build`.
