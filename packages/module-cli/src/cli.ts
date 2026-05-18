#!/usr/bin/env node
import { resolve } from 'node:path'
import { cwd } from 'node:process'

import { runBuild } from './build/run-build.js'
import { loadModuleDefinition } from './build/load-module.js'

async function main(): Promise<void> {
  const args = process.argv.slice(2)
  if (args.length === 0 || args[0] === '--help' || args[0] === '-h') {
    printHelp()
    return
  }

  const command = args[0]
  if (command !== 'build') {
    console.error(`Unknown command: ${command}`)
    printHelp()
    process.exit(1)
  }

  let entry = 'src/portaki.module.ts'
  let outDir = '.portaki'
  let manifestPath: string | null = 'portaki.module.json'
  let skipManifest = false

  for (let i = 1; i < args.length; i++) {
    if (args[i] === '--entry' && args[i + 1]) {
      entry = args[++i]
    } else if (args[i] === '--out' && args[i + 1]) {
      outDir = args[++i]
    } else if (args[i] === '--manifest' && args[i + 1]) {
      manifestPath = args[++i]
    } else if (args[i] === '--no-manifest') {
      skipManifest = true
    }
  }

  const entryPath = resolve(cwd(), entry)
  const outputPath = resolve(cwd(), outDir)
  const resolvedManifest =
    skipManifest || manifestPath == null ? null : resolve(cwd(), manifestPath)

  const module = await loadModuleDefinition(entryPath)
  const result = await runBuild(module, {
    outDir: outputPath,
    manifestPath: resolvedManifest,
  })

  console.log(`portaki-module: built ${result.module.id}@${result.module.version} → ${outputPath}`)
  if (result.module.backend) {
    console.log(
      `  schema ${result.module.backend.schemaVersion} — ${result.module.backend.schema.tables.length} table(s), migrations.bundle.json`,
    )
  }
  if (result.manifestPath) {
    console.log(
      result.manifestUpdated
        ? `  updated ${result.manifestPath} (hybrid merge: gateway fields)`
        : `  ${result.manifestPath} unchanged`,
    )
  }
}

function printHelp(): void {
  console.log(`@portaki/module-cli

Usage:
  portaki-module build [--entry src/portaki.module.ts] [--out .portaki] [--manifest portaki.module.json] [--no-manifest]

Outputs:
  .portaki/migrations.bundle.json   — DDL for modules DB (gitignored, no .sql in repo)
  portaki.module.json               — hybrid merge (catalogue kept; queries/commands/database/scopes updated)
  .portaki/backend/artifact.json    — Wasm metadata (AssemblyScript → gateway.wasm, phase 2)
`)
}

main().catch((error: unknown) => {
  console.error(error instanceof Error ? error.message : error)
  process.exit(1)
})
