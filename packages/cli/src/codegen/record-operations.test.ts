import { describe, expect, it } from 'vitest'

import {
  defineModule,
  jsonb,
  moduleSchema,
  propertyId,
  table,
  tenantId,
  timestamptz,
  uuidPrimaryKey,
  type HandlerContext,
} from '@portaki/sdk'

import { recordOperationsBundle } from './record-operations.js'

const rulesSchema = moduleSchema([
  table('content', 't_e_module_rules_ops', {
    columns: [
      uuidPrimaryKey(),
      propertyId(),
      tenantId(),
      jsonb('contentFr', { nullable: true }),
      timestamptz('updatedAt', { nullable: false, defaultSql: 'now()' }),
    ],
  }),
])

const rulesModule = defineModule({
  id: 'rules',
  label: { fr: 'R', en: 'R' },
  icon: 'scroll-text',
  version: '1.0.0',
  schema: rulesSchema,
  queries: {
    'rules.content': {
      scope: 'property:read',
      async handler(ctx: HandlerContext) {
        const row = await ctx.db
          .from('content')
          .where({ tenantId: ctx.tenantId, propertyId: ctx.propertyId })
          .one()
        return row ?? {}
      },
    },
  },
  commands: {
    'rules.content.save': {
      scope: 'host:property:write',
      async handler(ctx: HandlerContext, params: Record<string, unknown>) {
        const filters = { tenantId: ctx.tenantId, propertyId: ctx.propertyId }
        const existing = await ctx.db.from('content').where(filters).one()
        if (existing != null) {
          await ctx.db.from('content').where(filters).update({
            contentFr: params.contentFr ?? null,
          })
          return
        }
        await ctx.db.from('content').insert({ ...filters, contentFr: params.contentFr ?? null })
      },
    },
  },
  render: () => null,
})

describe('recordOperationsBundle', () => {
  it('whenNoData_thenEmptyOperations', async () => {
    const guestOnly = defineModule({
      id: 'guest',
      label: { fr: 'G', en: 'G' },
      icon: 'box',
      render: () => null,
    })
    const bundle = await recordOperationsBundle(guestOnly)
    expect(bundle.operations).toEqual({})
    expect(bundle.schemaVersion).toBe('0')
  })

  it('whenRulesModule_thenRecordsQueryAndCommandSteps', async () => {
    const bundle = await recordOperationsBundle(rulesModule)
    expect(bundle.moduleId).toBe('rules')
    expect(bundle.operations['rules.content']?.steps.length).toBeGreaterThan(0)
    expect(bundle.operations['rules.content.save']?.steps.length).toBeGreaterThan(0)
    const sql = bundle.operations['rules.content']?.steps[0]?.sql ?? ''
    expect(sql).toContain('t_e_module_rules_ops')
    expect(sql).not.toContain('REFERENCES')
  })
})
