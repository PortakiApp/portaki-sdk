import { describe, expect, it } from 'vitest'

import { jsonb, moduleSchema, table, uuidPrimaryKey } from '../schema/index.js'
import { findColumn, rowToCamel, valueForColumn } from './column-map.js'

const t = moduleSchema([
  table('items', 't_items', {
    columns: [uuidPrimaryKey(), jsonb('meta', { nullable: true })],
  }),
]).tables[0]!

describe('findColumn', () => {
  it('whenUnknownField_thenThrows', () => {
    expect(() => findColumn(t, 'missing')).toThrow(/Unknown field/)
  })
})

describe('rowToCamel', () => {
  it('whenJsonbString_thenParsesObject', () => {
    const row = rowToCamel({ meta: '{"k":1}' }, t)
    expect(row.meta).toEqual({ k: 1 })
  })

  it('whenInvalidJson_thenKeepsRawString', () => {
    const row = rowToCamel({ meta: 'not-json' }, t)
    expect(row.meta).toBe('not-json')
  })
})

describe('valueForColumn', () => {
  it('whenJsonbObject_thenStringifies', () => {
    const col = findColumn(t, 'meta')
    expect(valueForColumn(col, { a: 1 })).toBe('{"a":1}')
  })
})
