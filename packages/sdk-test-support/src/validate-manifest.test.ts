import { existsSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'

import { assertValidModuleManifest, validateModuleManifest } from './validate-manifest'

const schemaPath = join(dirname(fileURLToPath(import.meta.url)), '../../../schema/module.v1.json')

describe('validateModuleManifest', () => {
  it('loads module.v1.json from the sdk repo', () => {
    expect(existsSync(schemaPath)).toBe(true)
  })

  it('accepts a valid manifest with runtime and artifacts', () => {
    const manifest = {
      id: 'demo',
      name: { fr: 'Démo', en: 'Demo' },
      description: { fr: 'Desc', en: 'Desc' },
      version: '1.0.0',
      author: { name: 'Portaki', type: 'official' },
      icon: 'puzzle',
      type: 'official',
      license: 'AGPL-3.0',
      runtime: { backend: 'none', guest: 'bundled' },
      artifacts: { guestEsmUrl: 'https://example.com/bundle.js' },
    }
    assertValidModuleManifest(manifest, schemaPath)
  })

  it('rejects manifest without id', () => {
    const result = validateModuleManifest({ version: '1.0.0' }, schemaPath)
    expect(result.ok).toBe(false)
  })
})
