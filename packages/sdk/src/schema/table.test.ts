import { describe, expect, it } from 'vitest'

import { tenantId, uuidPrimaryKey } from './columns.js'
import { index, table, tenantPropertyIndex } from './table.js'

describe('table', () => {
  it('whenNoColumns_thenThrows', () => {
    expect(() => table('empty', 't_empty', { columns: [] })).toThrow(/at least one column/)
  })

  it('whenNoPrimaryKey_thenThrows', () => {
    expect(() =>
      table('no-pk', 't_no_pk', {
        columns: [tenantId()],
      }),
    ).toThrow(/primary key/)
  })

  it('whenValid_thenReturnsTableDef', () => {
    const def = table('ok', 't_ok', {
      columns: [uuidPrimaryKey(), tenantId()],
      indexes: [index('tenant', ['tenantId'])],
    })
    expect(def.logicalName).toBe('ok')
    expect(def.indexes).toHaveLength(1)
  })
})

describe('tenantPropertyIndex', () => {
  it('whenCalled_thenIndexesTenantAndProperty', () => {
    const idx = tenantPropertyIndex()
    expect(idx.columns).toEqual(['tenantId', 'propertyId'])
  })
})
