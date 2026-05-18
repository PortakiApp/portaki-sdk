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

  it('whenUnknownTable_thenThrows', () => {
    const db = createModuleDb(rulesSchema, {
      query: vi.fn(),
      queryOne: vi.fn(),
      execute: vi.fn(),
    })
    expect(() => db.from('missing')).toThrow(/Unknown table/)
  })

  it('whenInsert_thenCallsExecute', async () => {
    const execute = vi.fn().mockResolvedValue(1)
    const db = createModuleDb(rulesSchema, {
      query: vi.fn(),
      queryOne: vi.fn(),
      execute,
    })
    await db.from('content').insert({ tenantId: 't1', propertyId: 'p1' })
    expect(execute).toHaveBeenCalledWith(
      expect.stringContaining('INSERT INTO t_e_module_rules_content'),
      expect.any(Array),
    )
  })

  it('whenMany_thenMapsAllRows', async () => {
    const query = vi.fn().mockResolvedValue([
      { tenant_id: 't1', property_id: 'p1', content_fr: null, content_en: null },
    ])
    const db = createModuleDb(rulesSchema, {
      query,
      queryOne: vi.fn(),
      execute: vi.fn(),
    })
    const rows = await db.from('content').where({ tenantId: 't1' }).many()
    expect(rows).toHaveLength(1)
    expect(rows[0]?.tenantId).toBe('t1')
  })
})
