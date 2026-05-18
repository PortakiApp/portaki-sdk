import { describe, expect, it } from 'vitest'

import { uuidPrimaryKey } from './columns.js'
import { moduleSchema } from './module-schema.js'
import { table } from './table.js'

describe('moduleSchema', () => {
  it('whenNoTables_thenThrows', () => {
    expect(() => moduleSchema([])).toThrow(/at least one table/)
  })

  it('whenTablesProvided_thenWrapsTables', () => {
    const schema = moduleSchema([
      table('a', 't_a', { columns: [uuidPrimaryKey()] }),
    ])
    expect(schema.tables).toHaveLength(1)
  })
})
