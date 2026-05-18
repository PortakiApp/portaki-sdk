# Module authoring (SDK v2)

Greenfield model: **one TypeScript module**, no hand-written SQL in git, no Java backend.

## Packages

| Package | Role |
|---------|------|
| `@portaki/module-sdk` | Guest UI components + types |
| `@portaki/module-sdk/schema` | Declarative tables (`table`, `uuid`, `jsonb`, …) |
| `@portaki/module-sdk/gateway` | `GatewayContext`, `ModuleDatabase`, handler types |
| `@portaki/module-sdk/module` | `defineModule({ … })` — guest + backend |
| `@portaki/module-cli` | `portaki-module build` → `.portaki/` artifacts |

## Author layout

```
my-module/
  src/
    portaki.module.ts    # default export defineModule(...)
    RulesGuest.tsx
  portaki.module.json    # catalogue metadata (name, tags, hostSurfaces) — merge backend slice from build
  package.json
  .gitignore             # .portaki/
```

## Data access (no raw SQL)

Handlers use the schema-bound API only:

```ts
const row = await ctx.db.from('content').where({ tenantId: ctx.tenantId, propertyId: ctx.propertyId }).one()
await ctx.db.from('content').where({ tenantId, propertyId }).update({ contentFr, contentEn })
```

SQL is generated inside the SDK / host — not written by module authors.

## Example (`src/portaki.module.ts`)

See `sdk/examples/rules/src/portaki.module.ts`.

## Build

```bash
pnpm add -D @portaki/module-sdk @portaki/module-cli
pnpm exec portaki-module build
```

Generated:

- `.portaki/migrations.bundle.json` — gitignored; platform applies on modules DB (private network)
- **`portaki.module.json`** — hybrid merge: catalogue fields kept; `queries`, `commands`, `database`, `scopes` refreshed from code
- `.portaki/backend/artifact.json` — Wasm metadata (AssemblyScript → `gateway.wasm`, next phase)

Add `.portaki/` to `.gitignore`. Keep `portaki.module.json` in git for marketing / catalogue text.

## Runtime (platform, not author)

1. OCI image includes `gateway.wasm` + `migrations.bundle.json`
2. Java Extism runtime loads Wasm, applies pending revisions with lock
3. Host functions expose `ModuleDatabase` to Wasm (`portaki_db_*`)

Phase 2: `@portaki/module-cli` compiles handlers to **`gateway.wasm`** via **AssemblyScript** — see `docs/assemblyscript-backend.md`.

## Decisions (MVP)

- **No `.sql` in module repos** — SQL only inside `migrations.bundle.json` at build time
- **No FK** to `portaki-api` core tables (modules DB is separate)
- **Atlas** optional in dev to lint generated SQL; authors use schema DSL only
