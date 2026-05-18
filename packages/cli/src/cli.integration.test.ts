import { execFile } from 'node:child_process'
import { mkdtemp, readFile, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { promisify } from 'node:util'

import { describe, expect, it } from 'vitest'

const execFileAsync = promisify(execFile)
const cliDir = join(dirname(fileURLToPath(import.meta.url)), '..')
const repoRoot = join(cliDir, '../..')
const cliEntry = join(cliDir, 'dist/cli.js')
const rulesEntry = join(repoRoot, 'examples/rules/src/portaki.module.ts')

describe('portaki CLI integration', () => {
  it('whenBuildRulesExample_thenCreatesPortakiArtifacts', async () => {
    const outDir = await mkdtemp(join(tmpdir(), 'portaki-cli-'))
    try {
      const { stdout } = await execFileAsync(
        process.execPath,
        [cliEntry, 'build', '--entry', rulesEntry, '--out', outDir, '--no-manifest'],
        { cwd: repoRoot, env: process.env },
      )

      expect(stdout).toContain('portaki: built rules@')
      const migrations = JSON.parse(
        await readFile(join(outDir, 'migrations.bundle.json'), 'utf8'),
      ) as { moduleId: string }
      expect(migrations.moduleId).toBe('rules')
    } finally {
      await rm(outDir, { recursive: true, force: true })
    }
  })
})
