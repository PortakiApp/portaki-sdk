# Module authoring (SDK v2)

Greenfield model: **one TypeScript module project**, no hand-written SQL in git, no Java module code.

## Packages

| Package | Role |
|---------|------|
| `@portaki/sdk` | Everything for module source: `defineModule`, schema DSL, UI, hooks, handler types |
| `@portaki/sdk/runtime` | Optional subpath — same hooks as root (`usePortakiQuery`, `PortakiProvider`) for tree-shaking |
| `@portaki/sdk/server` | HMAC / Route Handlers Next (host app, not module livret) |
| `@portaki/cli` | `portaki build` → `.portaki/` artifacts (devDependency) |
| `@portaki/sdk-test-support` | Vitest / manifest (devDependency) |

`@portaki/sdk/build` is for the CLI only (operation recording at build time).

## Author layout

```
my-module/
  src/
    portaki.module.ts    # default export defineModule(...)
    GuestView.tsx
  portaki.module.json    # catalogue metadata — merge data slice from build
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
    "@portaki/cli": "^0.1.0",
    "@portaki/sdk-test-support": "^2.0.0"
  }
}
```

## Single import (typical)

```ts
import {
  defineModule,
  moduleSchema,
  table,
  uuidPrimaryKey,
  ModuleCard,
  usePortakiQuery,
} from '@portaki/sdk'
```

Hooks can also be imported from `@portaki/sdk/runtime` if you prefer a smaller mental surface on the main entry.

## Data access (no raw SQL)

```ts
async handler(ctx) {
  const row = await ctx.db
    .from('content')
    .where({ tenantId: ctx.tenantId, propertyId: ctx.propertyId })
    .one()
}
```

`content` = logical name in `table('content', 't_e_module_…', …)` — not the physical SQL table name.

Handler context: `HandlerContext` (`ctx.db`, `tenantId`, `propertyId`, scopes, …).

## Example

See `examples/rules/src/portaki.module.ts`.

## Build

```bash
pnpm add -D @portaki/cli
pnpm exec portaki build
```

Generated (gitignored under `.portaki/`): `migrations.bundle.json`, `operations.bundle.json`, hybrid merge into `portaki.module.json`.

## Migration from older packages

| Ancien | Nouveau |
|--------|---------|
| `@portaki/module-sdk` | `@portaki/sdk` |
| `@portaki/sdk` (hooks only, v0.5) | `@portaki/sdk` or `@portaki/sdk/runtime` |
| `@portaki/module-sdk/gateway` | `@portaki/sdk` (`defineModule`, `ctx.db`) |
| `@portaki/module-sdk/module` | `@portaki/sdk` (`defineModule`) |
| `@portaki/module-test-support` | `@portaki/sdk-test-support` |
| `@portaki/module-cli` | `@portaki/cli` (`portaki build`, was `portaki-module build`) |

Vitest preset still aliases `@portaki/module-sdk` → `@portaki/sdk` while the catalogue migrates.
