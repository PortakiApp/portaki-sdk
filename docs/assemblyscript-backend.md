# Backend module — AssemblyScript → Wasm

Module authors write **`defineModule` in TypeScript** (schema + handlers with `ctx.db.from(...)`).

The **CLI** will compile gateway handlers to **`gateway.wasm`** using **AssemblyScript** (syntax close to TypeScript, compiles to WebAssembly).

## Why AssemblyScript

- Runs inside **Extism** on the Java `portaki-module-runtime` (same as today’s shim).
- Sandboxed — no arbitrary npm in the host process.
- Predictable binary size and startup vs embedding Node.

## Author vs compiler

| You write (git) | Build produces (OCI / `.portaki/`) |
|-----------------|--------------------------------------|
| `src/portaki.module.ts` | `migrations.bundle.json` |
| `portaki.module.json` (catalogue) | `gateway.wasm` (from AS, phase 2) |
| — | hybrid-updated gateway fields in manifest |

You do **not** write `.sql` or `.as` by hand in the happy path — the CLI generates AS from handlers or ships a standard template.

## Host functions (Java)

Wasm calls the host for:

- `portaki_db_query` / `portaki_db_execute` — implements `ctx.db.from()` SQL inside the sandbox boundary
- `portaki_publish_event` — optional
- `portaki_log` — debugging

No `portaki_gateway_dispatch` + JAR in the target architecture.

## Status

- **Done**: schema DSL, `ctx.db` query builder (TS), migration bundle, hybrid manifest merge.
- **Next**: AS codegen + `gateway.wasm` in `portaki build`, runtime host DB functions + apply `migrations.bundle.json` on deploy.
