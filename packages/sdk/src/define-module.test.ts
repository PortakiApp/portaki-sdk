import { describe, expect, it } from 'vitest'

import { moduleSchema, propertyId, table, tenantId, uuidPrimaryKey } from './schema/index.js'
import { defineModule } from './define-module.js'

const minimalSchema = moduleSchema([
  table('content', 't_e_module_x_content', {
    columns: [uuidPrimaryKey(), propertyId(), tenantId()],
  }),
])

describe('defineModule', () => {
  it('whenGuestOnly_thenReturnsModuleWithoutData', () => {
    const mod = defineModule({
      id: 'guest-only',
      label: { fr: 'G', en: 'G' },
      icon: 'box',
      render: () => null,
    })

    expect(mod.data).toBeUndefined()
    expect(mod.id).toBe('guest-only')
  })

  it('whenQueriesWithoutSchema_thenAttachesConfigOnlyData', () => {
    const mod = defineModule({
      id: 'config-only',
      label: { fr: 'C', en: 'C' },
      icon: 'box',
      queries: {
        'config-only.read': { scope: 'host:property:read', handler: async () => ({}) },
      },
      renderHost: () => null,
    })

    expect(mod.data?.schema).toBeUndefined()
    expect(mod.data?.queries['config-only.read']).toBeDefined()
  })

  it('whenSchemaWithQueries_thenAttachesData', () => {
    const mod = defineModule({
      id: 'with-data',
      label: { fr: 'D', en: 'D' },
      icon: 'box',
      version: '2.0.0',
      schema: minimalSchema,
      queries: {
        'with-data.read': { scope: 'property:read', handler: async () => ({}) },
      },
      render: () => null,
    })

    expect(mod.data?.schemaVersion).toBe('2.0.0')
    expect(Object.keys(mod.data?.queries ?? {})).toEqual(['with-data.read'])
  })

  it('whenSchemaVersionOverride_thenUsesCustomVersion', () => {
    const mod = defineModule({
      id: 'sv',
      label: { fr: 'S', en: 'S' },
      icon: 'box',
      version: '1.0.0',
      schemaVersion: '3.1.0',
      schema: minimalSchema,
      commands: {
        'sv.save': { scope: 'host:property:write', handler: async () => {} },
      },
      render: () => null,
    })

    expect(mod.data?.schemaVersion).toBe('3.1.0')
  })
})
