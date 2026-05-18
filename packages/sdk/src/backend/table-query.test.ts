import { describe, expect, it, vi } from 'vitest'

import {
  index,
  jsonb,
  moduleSchema,
  propertyId,
  table,
  tenantId,
  timestamptz,
  uuidPrimaryKey,
} from '../schema/index.js'
import { createModuleDb } from './table-query.js'
import type { ModuleDatabase } from './types.js'

const rulesSchema = moduleSchema([
  table('content', 't_e_module_rules_content', {
    columns: [
      uuidPrimaryKey(),
      propertyId(),
      tenantId(),
      jsonb('contentFr', { nullable: true }),
      jsonb('contentEn', { nullable: true }),
      timestamptz('updatedAt', { nullable: false, defaultSql: 'now()' }),
    ],
    indexes: [index('tenant', ['tenantId'])],
  }),
])

describe('createModuleDb', () => {
  it('whenFindOne_thenParameterizedSelectWithoutAuthorSql', async () => {
    const queryOne = vi.fn().mockResolvedValue({
      content_fr: '{"a":1}',
      content_en: null,
      property_id: 'p1',
      tenant_id: 't1',
    })
    const database: ModuleDatabase = {
      query: vi.fn(),
      queryOne,
      execute: vi.fn(),
    }
    const db = createModuleDb(rulesSchema, database)
    const row = await db
      .from('content')
      .where({ tenantId: 't1', propertyId: 'p1' })
      .one()

    expect(queryOne).toHaveBeenCalledWith(
      expect.stringContaining('FROM t_e_module_rules_content'),
      ['t1', 'p1'],
    )
    expect(queryOne.mock.calls[0][0]).not.toContain('REFERENCES')
    expect(row?.contentFr).toEqual({ a: 1 })
  })
})
