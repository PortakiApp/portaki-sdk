import { describe, expect, it } from 'vitest'

import { defineModule } from '@portaki/module-sdk/module'
import { moduleSchema, propertyId, table, tenantId, uuidPrimaryKey } from '@portaki/module-sdk/schema'

import { mergeHybridManifest } from './merge-hybrid-manifest.js'

describe('mergeHybridManifest', () => {
  it('whenCatalogueFieldsPresent_thenPreservesDescriptionAndRefreshesGateway', () => {
    const module = defineModule({
      id: 'rules',
      label: { fr: 'R', en: 'R' },
      icon: 'scroll-text',
      version: '1.0.0',
      schema: moduleSchema([
        table('content', 't_e_module_rules_content', {
          columns: [uuidPrimaryKey(), propertyId(), tenantId()],
        }),
      ]),
      queries: {
        'rules.content': {
          scope: 'property:read',
          handler: async () => ({}),
        },
      },
      render: () => null,
    })

    const existing = {
      id: 'rules',
      description: { fr: 'Catalogue', en: 'Catalogue' },
      version: '0.9.0',
      scopes: ['stay:read'],
      queries: [],
    }

    const { merged } = mergeHybridManifest(existing, module)
    expect(merged.description).toEqual({ fr: 'Catalogue', en: 'Catalogue' })
    expect(merged.version).toBe('1.0.0')
    expect(merged.queries).toHaveLength(1)
    expect(merged.scopes).toEqual(['property:read', 'stay:read'])
  })
})
