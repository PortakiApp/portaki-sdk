import { describe, expect, it } from 'vitest'

import {
  index,
  jsonb,
  moduleSchema,
  propertyId,
  table,
  tenantId,
  timestamptz,
  uuidPrimaryKey,
} from '@portaki/sdk'

import { generateSchemaMigrations } from './generate-schema-sql.js'

describe('generateSchemaMigrations', () => {
  it('whenRulesSchema_thenNoCoreForeignKeys', () => {
    const schema = moduleSchema([
      table('content', 't_e_module_rules_content', {
        columns: [
          uuidPrimaryKey(),
          propertyId(),
          tenantId(),
          jsonb('contentFr', { nullable: true }),
          jsonb('contentEn', { nullable: true }),
          timestamptz('updatedAt', { defaultSql: 'now()' }),
        ],
        indexes: [index('tenant', ['tenantId'])],
      }),
    ])

    const revisions = generateSchemaMigrations('rules', '1.0.0', schema)
    expect(revisions).toHaveLength(1)
    const sql = revisions[0].sql
    expect(sql).toContain('CREATE TABLE IF NOT EXISTS t_e_module_rules_content')
    expect(sql).not.toContain('REFERENCES t_e_properties')
    expect(sql).not.toContain('REFERENCES t_e_tenants')
    expect(sql).toContain('property_id UUID NOT NULL UNIQUE')
  })
})
