import { describe, expect, it } from 'vitest'

import { jsonb, moduleSchema, propertyId, table, tenantId, uuidPrimaryKey } from '../schema/index.js'
import { buildInsert, buildSelect, buildUpdate } from './sql-builder.js'
import { findColumn } from './column-map.js'

const contentTable = moduleSchema([
  table('content', 't_e_test_content', {
    columns: [
      uuidPrimaryKey(),
      propertyId(),
      tenantId(),
      jsonb('contentFr', { nullable: true }),
    ],
  }),
]).tables[0]!

describe('buildSelect', () => {
  it('whenNoFilters_thenSelectAllColumns', () => {
    const { sql, params } = buildSelect(contentTable, [])
    expect(sql).toContain('SELECT id, property_id, tenant_id, content_fr')
    expect(sql).toContain('FROM t_e_test_content')
    expect(params).toEqual([])
  })

  it('whenWhereClause_thenParameterized', () => {
    const tenantCol = findColumn(contentTable, 'tenantId')
    const { sql, params } = buildSelect(contentTable, [{ column: tenantCol, value: 't1' }], 1)
    expect(sql).toContain('WHERE tenant_id = $1')
    expect(sql).toContain('LIMIT 1')
    expect(params).toEqual(['t1'])
  })
})

describe('buildInsert', () => {
  it('whenJsonbValue_thenCastPlaceholder', () => {
    const { sql, params } = buildInsert(contentTable, {
      tenantId: 't1',
      propertyId: 'p1',
      contentFr: { type: 'doc' },
    })
    expect(sql).toContain('::jsonb')
    expect(params[2]).toBe('{"type":"doc"}')
  })

  it('whenUnknownField_thenThrows', () => {
    expect(() => buildInsert(contentTable, { unknown: 1 })).toThrow(/Unknown field/)
  })
})

describe('buildUpdate', () => {
  it('whenSetAndWhere_thenParameterizedUpdate', () => {
    const tenantCol = findColumn(contentTable, 'tenantId')
    const { sql, params } = buildUpdate(
      contentTable,
      { contentFr: null },
      [{ column: tenantCol, value: 't1' }],
    )
    expect(sql).toContain('UPDATE t_e_test_content SET')
    expect(sql).toContain('WHERE tenant_id = $')
    expect(params).toHaveLength(2)
  })
})
