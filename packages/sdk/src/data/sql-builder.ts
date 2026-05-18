import type { ColumnDef, TableDef } from '../schema/types.js'

import { valueForColumn } from './column-map.js'

export type WhereClause = {
  readonly column: ColumnDef
  readonly value: unknown
}

export function buildSelect(
  table: TableDef,
  conditions: readonly WhereClause[],
  limit?: number,
): { sql: string; params: unknown[] } {
  const params: unknown[] = []
  let sql = `SELECT ${table.columns.map((c) => c.sqlName).join(', ')} FROM ${table.tableName}`
  if (conditions.length > 0) {
    const parts = conditions.map((c, index) => {
      params.push(valueForColumn(c.column, c.value))
      return `${c.column.sqlName} = $${index + 1}`
    })
    sql += ` WHERE ${parts.join(' AND ')}`
  }
  if (limit != null) {
    sql += ` LIMIT ${limit}`
  }
  return { sql, params }
}

export function buildInsert(
  table: TableDef,
  values: Record<string, unknown>,
): { sql: string; params: unknown[] } {
  const columns: ColumnDef[] = []
  const params: unknown[] = []
  for (const [fieldName, value] of Object.entries(values)) {
    const column = table.columns.find((c) => c.name === fieldName)
    if (column == null) {
      throw new Error(`Unknown field "${fieldName}" on table "${table.logicalName}"`)
    }
    if (column.primaryKey && column.defaultSql != null && value === undefined) {
      continue
    }
    columns.push(column)
    params.push(valueForColumn(column, value))
  }
  if (columns.length === 0) {
    throw new Error(`insert on "${table.logicalName}" requires at least one column`)
  }
  const placeholders = columns.map((c, index) => {
    if (c.type === 'jsonb') {
      return `$${index + 1}::jsonb`
    }
    return `$${index + 1}`
  })
  const sql = `INSERT INTO ${table.tableName} (${columns.map((c) => c.sqlName).join(', ')}) VALUES (${placeholders.join(', ')})`
  return { sql, params }
}

export function buildUpdate(
  table: TableDef,
  values: Record<string, unknown>,
  conditions: readonly WhereClause[],
): { sql: string; params: unknown[] } {
  const params: unknown[] = []
  const setParts: string[] = []
  for (const [fieldName, value] of Object.entries(values)) {
    const column = table.columns.find((c) => c.name === fieldName)
    if (column == null) {
      throw new Error(`Unknown field "${fieldName}" on table "${table.logicalName}"`)
    }
    if (column.primaryKey) {
      continue
    }
    params.push(valueForColumn(column, value))
    const index = params.length
    if (column.type === 'jsonb') {
      setParts.push(`${column.sqlName} = $${index}::jsonb`)
    } else {
      setParts.push(`${column.sqlName} = $${index}`)
    }
  }
  if (setParts.length === 0) {
    throw new Error(`update on "${table.logicalName}" requires at least one column`)
  }
  let sql = `UPDATE ${table.tableName} SET ${setParts.join(', ')}`
  if (conditions.length > 0) {
    const whereParts = conditions.map((c) => {
      params.push(valueForColumn(c.column, c.value))
      return `${c.column.sqlName} = $${params.length}`
    })
    sql += ` WHERE ${whereParts.join(' AND ')}`
  }
  return { sql, params }
}
