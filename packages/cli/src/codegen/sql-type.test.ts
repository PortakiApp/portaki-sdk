import { describe, expect, it } from 'vitest'

import { jsonb, timestamptz, uuidPrimaryKey } from '@portaki/sdk'

import { sqlType } from './sql-type.js'

describe('sqlType', () => {
  it('whenPrimaryKeyUuid_thenIncludesPrimaryKeyAndDefault', () => {
    const sql = sqlType(uuidPrimaryKey())
    expect(sql).toContain('UUID')
    expect(sql).toContain('PRIMARY KEY')
    expect(sql).toContain('gen_random_uuid()')
  })

  it('whenNullableJsonb_thenOmitsNotNull', () => {
    const sql = sqlType(jsonb('payload', { nullable: true }))
    expect(sql).toBe('JSONB')
    expect(sql).not.toContain('NOT NULL')
  })

  it('whenRequiredTimestamptz_thenNotNull', () => {
    const sql = sqlType(timestamptz('updatedAt', { defaultSql: 'now()' }))
    expect(sql).toContain('TIMESTAMPTZ')
    expect(sql).toContain('NOT NULL')
    expect(sql).toContain('DEFAULT now()')
  })
})
