import { describe, expect, it } from 'vitest'

import { toMigrationBundle } from './migration-bundle.js'

describe('toMigrationBundle', () => {
  it('whenRevisionsProvided_thenMapsModuleMetadata', () => {
    const bundle = toMigrationBundle('rules', '1.0.0', [
      { revision: '001', sql: 'CREATE TABLE t();' },
    ])
    expect(bundle.moduleId).toBe('rules')
    expect(bundle.schemaVersion).toBe('1.0.0')
    expect(bundle.revisions[0]).toEqual({ revision: '001', sql: 'CREATE TABLE t();' })
  })
})
