import { describe, expect, it } from 'vitest'

import { jsonb, propertyId, text, uuidPrimaryKey } from './columns.js'

describe('column helpers', () => {
  it('whenCamelCaseName_thenSnakeSqlName', () => {
    expect(jsonb('contentFr').sqlName).toBe('content_fr')
    expect(text('displayName').sqlName).toBe('display_name')
  })

  it('whenUuidPrimaryKey_thenDefaultGenRandomUuid', () => {
    const col = uuidPrimaryKey()
    expect(col.primaryKey).toBe(true)
    expect(col.defaultSql).toBe('gen_random_uuid()')
  })

  it('whenPropertyId_thenUniqueUuid', () => {
    const col = propertyId()
    expect(col.unique).toBe(true)
    expect(col.sqlName).toBe('property_id')
  })
})
