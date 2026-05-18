# Module authoring (SDK v2)

Greenfield model: **one TypeScript module**, no hand-written SQL in git, no Java backend.

## Packages

| Package | Role |
|---------|------|
| `@portaki/sdk` | Module UI (`definePortakiModule`), components, types |
| `@portaki/sdk/runtime` | Livret invité : `usePortakiQuery`, `PortakiProvider`, slots |
| `@portaki/sdk/server` | HMAC / Route Handlers Next |
| `@portaki/sdk/schema` | Tables déclaratives (`table`, `uuid`, `jsonb`, …) |
| `@portaki/sdk/backend` | Handlers, `GatewayContext`, `ModuleDatabase` |
| `@portaki/sdk/author` | `defineModule({ … })` — guest + backend |
| `@portaki/module-cli` | `portaki-module build` → `.portaki/` artifacts |
| `@portaki/sdk-test-support` | Vitest / manifest (devDependencies) |

## Author layout

```
my-module/
  src/
    portaki.module.ts    # default export defineModule(...)
    GuestView.tsx
  portaki.module.json    # catalogue metadata — merge backend slice from build
  package.json
  .gitignore             # .portaki/
```

## Dependencies (module)

```json
{
  "dependencies": {
    "@portaki/sdk": "^2.0.0"
  },
  "devDependencies": {
    "@portaki/module-cli": "^0.1.0",
    "@portaki/sdk-test-support": "^2.0.0"
  }
}
```

Dans le code :

```ts
import { defineModule, ModuleCard } from '@portaki/sdk'
import { usePortakiQuery } from '@portaki/sdk/runtime'
```

## Data access (no raw SQL)

```ts
const row = await ctx.db.from('content').where({ tenantId: ctx.tenantId, propertyId: ctx.propertyId }).one()
```

`content` = nom logique dans `table('content', 't_e_module_…', …)` — pas le nom SQL physique.

## Example

See `examples/rules/src/portaki.module.ts`.

## Build

```bash
pnpm add -D @portaki/sdk @portaki/module-cli
pnpm exec portaki-module build
```

Generated (gitignored under `.portaki/`): `migrations.bundle.json`, `operations.bundle.json`, manifest merge.

## Subpath map (ex-« ponts »)

| Ancien | Nouveau |
|--------|---------|
| `@portaki/module-sdk` | `@portaki/sdk` |
| `@portaki/sdk` (hooks seuls, v0.5) | `@portaki/sdk/runtime` |
| `@portaki/module-sdk/gateway` | `@portaki/sdk/backend` |
| `@portaki/module-sdk/module` | `@portaki/sdk/author` |
| `@portaki/module-test-support` | `@portaki/sdk-test-support` |

Vitest preset accepte encore `@portaki/module-sdk` en alias le temps de la migration du catalogue.
