/**
 * Reference full-stack module (guest + backend, no raw SQL).
 */
import { defineModule } from '@portaki/sdk/author'
import {
  index,
  jsonb,
  moduleSchema,
  propertyId,
  table,
  tenantId,
  timestamptz,
  uuidPrimaryKey,
} from '@portaki/sdk/schema'

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

export default defineModule({
  id: 'rules',
  label: { fr: 'Règlement intérieur', en: 'House rules' },
  description: {
    fr: 'Consignes du logement pour les voyageurs.',
    en: 'House rules for guests.',
  },
  icon: 'scroll-text',
  version: '1.0.0',
  schemaVersion: '1.0.0',
  schema: rulesSchema,

  queries: {
    'rules.content': {
      scope: 'property:read',
      description: {
        fr: 'Contenu du règlement.',
        en: 'House rules content.',
      },
      async handler(ctx) {
        const row = await ctx.db
          .from('content')
          .where({ tenantId: ctx.tenantId, propertyId: ctx.propertyId })
          .one()
        if (row == null) {
          return {}
        }
        const result: Record<string, unknown> = {}
        if (row.contentFr != null) {
          result.contentFr = row.contentFr
        }
        if (row.contentEn != null) {
          result.contentEn = row.contentEn
        }
        return result
      },
    },
  },

  commands: {
    'rules.content.save': {
      scope: 'host:property:write',
      description: {
        fr: 'Enregistrer le règlement.',
        en: 'Save house rules content.',
      },
      async handler(ctx, params) {
        const filters = { tenantId: ctx.tenantId, propertyId: ctx.propertyId }
        const existing = await ctx.db.from('content').where(filters).one()
        const payload = {
          contentFr: params.contentFr ?? null,
          contentEn: params.contentEn ?? null,
        }
        if (existing != null) {
          await ctx.db.from('content').where(filters).update(payload)
          return
        }
        await ctx.db.from('content').insert({
          ...filters,
          ...payload,
        })
      },
    },
  },

  render: () => null,
})
