import { describe, expect, it } from 'vitest'

import {
  defineModule,
  moduleSchema,
  propertyId,
  table,
  tenantId,
  uuidPrimaryKey,
} from '@portaki/sdk'

import { moduleDataManifestSlice } from './manifest-data.js'

describe('moduleDataManifestSlice', () => {
  it('whenNoData_thenEmptyScopes', () => {
    const mod = defineModule({
      id: 'ui-only',
      label: { fr: 'U', en: 'U' },
      icon: 'box',
      render: () => null,
    })
    expect(moduleDataManifestSlice(mod)).toEqual({ scopes: [] })
  })

  it('whenQueriesAndCommands_thenCollectsScopes', () => {
    const schema = moduleSchema([
      table('content', 't_e_manifest_content', {
        columns: [uuidPrimaryKey(), propertyId(), tenantId()],
      }),
    ])
    const mod = defineModule({
      id: 'full',
      label: { fr: 'F', en: 'F' },
      icon: 'box',
      schema,
      queries: {
        'full.read': { scope: 'property:read', handler: async () => ({}) },
      },
      commands: {
        'full.save': { scope: 'host:property:write', handler: async () => {} },
      },
      render: () => null,
    })

    const slice = moduleDataManifestSlice(mod)
    expect(slice.scopes).toEqual(['host:property:write', 'property:read'])
    expect(slice.database?.schemaVersion).toBeDefined()
    expect(slice.queries?.[0]?.name).toBe('full.read')
    expect(slice.commands?.[0]?.name).toBe('full.save')
  })
})
