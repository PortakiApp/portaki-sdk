import { describe, expect, it, vi } from 'vitest'

import { moduleSchema, propertyId, table, tenantId, uuidPrimaryKey } from '../schema/index.js'
import { createHandlerContext } from './create-handler-context.js'

const schema = moduleSchema([
  table('content', 't_e_ctx_content', {
    columns: [uuidPrimaryKey(), propertyId(), tenantId()],
  }),
])

describe('createHandlerContext', () => {
  it('whenCreated_thenExposesDbAndScopeFields', async () => {
    const queryOne = vi.fn().mockResolvedValue(null)
    const ctx = createHandlerContext({
      moduleId: 'rules',
      moduleVersion: '1.0.0',
      tenantId: 't1',
      propertyId: 'p1',
      stayId: 's1',
      scopes: ['property:read'],
      config: { flag: true },
      schema,
      database: { query: vi.fn(), queryOne, execute: vi.fn() },
    })

    expect(ctx.moduleId).toBe('rules')
    expect(ctx.config.flag).toBe(true)
    await ctx.db.from('content').where({ tenantId: 't1' }).one()
    expect(queryOne).toHaveBeenCalled()
  })
})
