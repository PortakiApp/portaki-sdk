import { describe, expect, it } from 'vitest'

import { moduleSchema, propertyId, table, tenantId, uuidPrimaryKey } from '../schema/index.js'
import { createRecordingModuleDb } from './recording-db.js'

const schema = moduleSchema([
  table('content', 't_e_rec_content', {
    columns: [uuidPrimaryKey(), propertyId(), tenantId()],
  }),
])

describe('RecordingModuleDatabase', () => {
  it('whenDbOperations_thenRecordsCalls', async () => {
    const { db, database } = createRecordingModuleDb(schema)
    await db.from('content').where({ tenantId: 't1' }).one()
    await db.from('content').insert({ tenantId: 't1', propertyId: 'p1' })

    expect(database.calls).toHaveLength(2)
    expect(database.calls[0]?.kind).toBe('queryOne')
    expect(database.calls[1]?.kind).toBe('execute')
  })

  it('whenSeededQueryOne_thenReturnsQueuedRow', async () => {
    const { database } = createRecordingModuleDb(schema)
    database.seedQueryOneResponses({ id: 'row-1' })
    const row = await database.queryOne('SELECT 1')
    expect(row).toEqual({ id: 'row-1' })
  })
})
