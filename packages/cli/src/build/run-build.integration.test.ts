import { mkdtemp, readFile, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

import { describe, expect, it } from 'vitest'

import { loadModuleDefinition } from './load-module.js'
import { runBuild } from './run-build.js'

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../../../..')
const rulesEntry = join(repoRoot, 'examples/rules/src/portaki.module.ts')
const rulesManifest = join(repoRoot, 'examples/rules/portaki.module.json')

describe('runBuild integration (examples/rules)', () => {
  it('whenRulesExample_thenWritesMigrationAndOperationsBundles', async () => {
    const outDir = await mkdtemp(join(tmpdir(), 'portaki-build-'))
    try {
      const module = await loadModuleDefinition(rulesEntry)
      const result = await runBuild(module, {
        outDir,
        manifestPath: rulesManifest,
        entryPath: null,
      })

      expect(result.module.id).toBe('rules')
      expect(result.module.data).toBeDefined()

      const migrationsRaw = await readFile(join(outDir, 'migrations.bundle.json'), 'utf8')
      const migrations = JSON.parse(migrationsRaw) as {
        moduleId: string
        revisions: { sql: string }[]
      }
      expect(migrations.moduleId).toBe('rules')
      expect(migrations.revisions[0]?.sql).toContain('t_e_module_rules_content')

      const operationsRaw = await readFile(join(outDir, 'operations.bundle.json'), 'utf8')
      const operations = JSON.parse(operationsRaw) as {
        operations: Record<string, { steps: { sql: string }[] }>
      }
      expect(operations.operations['rules.content']?.steps.length).toBeGreaterThan(0)
      expect(operations.operations['rules.content.save']?.steps.length).toBeGreaterThan(0)

      const artifactRaw = await readFile(join(outDir, 'backend', 'artifact.json'), 'utf8')
      const artifact = JSON.parse(artifactRaw) as { moduleId: string; runtime: string }
      expect(artifact.moduleId).toBe('rules')
      expect(artifact.runtime).toBe('wasm')
    } finally {
      await rm(outDir, { recursive: true, force: true })
    }
  })
})
