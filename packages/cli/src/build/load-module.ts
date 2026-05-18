import { createRequire } from 'node:module'
import { mkdtemp, rm } from 'node:fs/promises'
import { existsSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'

import type { PortakiFullModule } from '@portaki/sdk'
import * as esbuild from 'esbuild'

function resolveSdkEntry(entryPath: string): string {
  const monorepoCandidates: string[] = []
  const fromCwd = join(process.cwd(), '../../../portaki-sdk/packages/sdk/src/index.ts')
  if (existsSync(fromCwd)) {
    monorepoCandidates.push(fromCwd)
  }
  let dir = dirname(entryPath)
  for (let depth = 0; depth < 8; depth++) {
    const sdkSrc = join(dir, 'portaki-sdk/packages/sdk/src/index.ts')
    if (existsSync(sdkSrc)) {
      monorepoCandidates.push(sdkSrc)
    }
    const parent = dirname(dir)
    if (parent === dir) {
      break
    }
    dir = parent
  }
  const installedCandidates: string[] = []
  try {
    installedCandidates.push(createRequire(entryPath).resolve('@portaki/sdk'))
  } catch {
    // module not installed locally
  }
  try {
    installedCandidates.push(createRequire(process.cwd()).resolve('@portaki/sdk'))
  } catch {
    // ignore
  }
  const candidates = [...monorepoCandidates, ...installedCandidates]
  const resolved = candidates.find((path) => existsSync(path))
  if (resolved == null) {
    throw new Error(
      `Cannot resolve @portaki/sdk for ${entryPath}. Install the package or build portaki-sdk (packages/sdk/dist).`,
    )
  }
  return resolved
}

function assertModule(mod: unknown, entryPath: string): PortakiFullModule {
  const definition = mod as PortakiFullModule | undefined
  if (definition == null || typeof definition !== 'object' || typeof definition.id !== 'string') {
    throw new Error(`Module entry must export default defineModule(...): ${entryPath}`)
  }
  return definition
}

export async function loadModuleDefinition(entryPath: string): Promise<PortakiFullModule> {
  const tempDir = await mkdtemp(join(tmpdir(), 'portaki-module-load-'))
  const outfile = join(tempDir, 'entry.mjs')
  const isTsx = entryPath.endsWith('.tsx') || entryPath.endsWith('.jsx')
  try {
    const sdkEntry = resolveSdkEntry(entryPath)
    const cliRequire = createRequire(import.meta.url)
    await esbuild.build({
      entryPoints: [entryPath],
      bundle: true,
      platform: 'node',
      format: 'esm',
      outfile,
      logLevel: 'silent',
      ...(isTsx ? { jsx: 'automatic' } : {}),
      alias: {
        '@portaki/sdk': sdkEntry,
        react: cliRequire.resolve('react'),
        'react-dom': cliRequire.resolve('react-dom'),
        'react/jsx-runtime': cliRequire.resolve('react/jsx-runtime'),
        'react/jsx-dev-runtime': cliRequire.resolve('react/jsx-dev-runtime'),
      },
    })
    const loaded = (await import(outfile)) as Record<string, unknown>
    return assertModule(loaded.default ?? loaded.module, entryPath)
  } finally {
    await rm(tempDir, { recursive: true, force: true })
  }
}
