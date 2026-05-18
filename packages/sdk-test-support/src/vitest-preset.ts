/**
 * @file vitest-preset.ts
 * @brief Vitest configuration preset for `@portaki/module-*` packages.
 *
 * @details Import via `@portaki/sdk-test-support/vitest` — not the main package entry.
 *
 * @copyright Portaki — SPDX-License-Identifier: MIT
 * @addtogroup sdk_test_support
 * @{
 */

import { existsSync } from 'node:fs'
import { createRequire } from 'node:module'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

import { defineConfig } from 'vitest/config'

const setupFile = join(dirname(fileURLToPath(import.meta.url)), 'vitest-setup.js')

function resolvePackageSrcEntry(moduleRoot: string, packageName: string, entry = 'src/index.ts'): string {
  const linkedSrc = join(moduleRoot, 'node_modules', packageName, entry)
  if (existsSync(linkedSrc)) {
    return linkedSrc
  }
  try {
    const require = createRequire(join(moduleRoot, 'package.json'))
    const resolved = require.resolve(packageName)
    const srcSibling = join(dirname(resolved), '..', entry)
    if (existsSync(srcSibling)) {
      return srcSibling
    }
    return resolved
  } catch {
    const presetDir = dirname(fileURLToPath(import.meta.url))
    const fallbackByPackage: Record<string, string> = {
      '@portaki/sdk': '../../sdk/src/index.ts',
      '@portaki/sdk/runtime': '../../sdk/src/runtime/index.ts',
      '@portaki/module-sdk': '../../sdk/src/index.ts',
    }
    const rel = fallbackByPackage[packageName] ?? entry
    return join(presetDir, rel)
  }
}

/**
 * @brief Returns a Vitest config rooted at the calling module package.
 * @param moduleEntryUrl Pass `import.meta.url` from the module's `vitest.config.ts`.
 *
 * @example
 * ```ts
 * import portakiModuleVitestConfig from '@portaki/sdk-test-support/vitest'
 * export default portakiModuleVitestConfig(import.meta.url)
 * ```
 */
export function portakiModuleVitestConfig(moduleEntryUrl: string) {
  const moduleRoot = dirname(fileURLToPath(moduleEntryUrl))
  const sdkEntry = resolvePackageSrcEntry(moduleRoot, '@portaki/sdk')
  const runtimeEntry = resolvePackageSrcEntry(moduleRoot, '@portaki/sdk/runtime', 'src/runtime/index.ts')
  return defineConfig({
    root: moduleRoot,
    resolve: {
      alias: {
        '@portaki/sdk/runtime': runtimeEntry,
        '@portaki/sdk': sdkEntry,
        '@portaki/module-sdk': sdkEntry,
      },
    },
    test: {
      environment: 'jsdom',
      setupFiles: [setupFile],
      include: ['src/**/*.test.ts', 'src/**/*.test.tsx'],
      passWithNoTests: false,
      server: {
        deps: {
          inline: ['@portaki/sdk', '@portaki/module-sdk', '@portaki/sdk-test-support'],
        },
      },
    },
  })
}

export default portakiModuleVitestConfig

/** @} */
