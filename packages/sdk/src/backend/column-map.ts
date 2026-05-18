import type { ColumnDef, TableDef } from '../schema/types.js'

export function findColumn(table: TableDef, fieldName: string): ColumnDef {
  const column = table.columns.find((c) => c.name === fieldName)
  if (column == null) {
    throw new Error(`Unknown field "${fieldName}" on table "${table.logicalName}"`)
  }
  return column
}

export function rowToCamel(row: Record<string, unknown>, table: TableDef): Record<string, unknown> {
  const result: Record<string, unknown> = {}
  for (const column of table.columns) {
    const raw = row[column.sqlName]
    if (raw === undefined) {
      continue
    }
    if (column.type === 'jsonb' && typeof raw === 'string') {
      try {
        result[column.name] = JSON.parse(raw)
      } catch {
        result[column.name] = raw
      }
    } else {
      result[column.name] = raw
    }
  }
  return result
}

export function valueForColumn(column: ColumnDef, value: unknown): unknown {
  if (value === undefined) {
    return undefined
  }
  if (column.type === 'jsonb' && value != null && typeof value === 'object') {
    return JSON.stringify(value)
  }
  return value
}
