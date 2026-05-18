#!/usr/bin/env node
import { resolve } from 'node:path'
import { cwd } from 'node:process'

import type { PortakiFullModule } from '@portaki/sdk'

import { runBuild } from './build/run-build.js'
import { loadModuleDefinition } from './build/load-module.js'

function hasModuleExtension(module: PortakiFullModule): boolean {
  return (
    Object.keys(module.hostActions ?? {}).length > 0 ||
    Object.keys(module.eventHandlers ?? {}).length > 0
  )
}

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
  let extensionEntry: string | null = null

  for (let i = 1; i < args.length; i++) {
    if (args[i] === '--entry' && args[i + 1]) {
      entry = args[++i]
    } else if (args[i] === '--extension-entry' && args[i + 1]) {
      extensionEntry = args[++i]
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
  const extensionEntryPath =
    extensionEntry != null
      ? resolve(cwd(), extensionEntry)
      : hasModuleExtension(module)
        ? entryPath
        : null

  const result = await runBuild(module, {
    outDir: outputPath,
    manifestPath: resolvedManifest,
    entryPath: extensionEntryPath,
  })

  console.log(`portaki: built ${result.module.id}@${result.module.version} → ${outputPath}`)
  if (result.module.data) {
    const tableCount = result.module.data.schema?.tables.length ?? 0
    console.log(
      `  schema ${result.module.data.schemaVersion} — ${tableCount} table(s), operations bundle`,
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
  console.log(`@portaki/cli

Usage:
  portaki build [--entry src/portaki.module.ts] [--extension-entry src/module-extension.ts] [--out .portaki] [--manifest portaki.module.json] [--no-manifest]

Outputs:
  .portaki/migrations.bundle.json   — DDL for modules DB (gitignored, no .sql in repo)
  portaki.module.json               — hybrid merge (catalogue kept; queries/commands/database/scopes updated)
  .portaki/backend/artifact.json    — Wasm artifact metadata (platform bundle layout)
`)
}

main().catch((error: unknown) => {
  console.error(error instanceof Error ? error.message : error)
  process.exit(1)
})
