import type { ModuleSchemaDef, TableDef } from '../schema/types.js'

import { findColumn, rowToCamel } from './column-map.js'
import type { ModuleDatabase } from './types.js'
import { buildInsert, buildSelect, buildUpdate, type WhereClause } from './sql-builder.js'

export class TableQuery {
  private readonly conditions: WhereClause[] = []

  constructor(
    private readonly table: TableDef,
    private readonly database: ModuleDatabase,
  ) {}

  where(filters: Record<string, unknown>): TableQuery {
    const next = new TableQuery(this.table, this.database)
    next.conditions.push(...this.conditions)
    for (const [fieldName, value] of Object.entries(filters)) {
      next.conditions.push({
        column: findColumn(this.table, fieldName),
        value,
      })
    }
    return next
  }

  async one(): Promise<Record<string, unknown> | null> {
    const { sql, params } = buildSelect(this.table, this.conditions, 1)
    const row = await this.database.queryOne<Record<string, unknown>>(sql, params)
    if (row == null) {
      return null
    }
    return rowToCamel(row, this.table)
  }

  async many(): Promise<Record<string, unknown>[]> {
    const { sql, params } = buildSelect(this.table, this.conditions)
    const rows = await this.database.query<Record<string, unknown>>(sql, params)
    return rows.map((row) => rowToCamel(row, this.table))
  }

  async insert(values: Record<string, unknown>): Promise<void> {
    const { sql, params } = buildInsert(this.table, values)
    await this.database.execute(sql, params)
  }

  async update(values: Record<string, unknown>): Promise<number> {
    const { sql, params } = buildUpdate(this.table, values, this.conditions)
    return this.database.execute(sql, params)
  }
}

export type ModuleDb = {
  from(logicalTableName: string): TableQuery
}

export function createModuleDb(schema: ModuleSchemaDef, database: ModuleDatabase): ModuleDb {
  return {
    from(logicalTableName: string): TableQuery {
      const table = schema.tables.find((t) => t.logicalName === logicalTableName)
      if (table == null) {
        throw new Error(`Unknown table "${logicalTableName}" in module schema`)
      }
      return new TableQuery(table, database)
    },
  }
}
