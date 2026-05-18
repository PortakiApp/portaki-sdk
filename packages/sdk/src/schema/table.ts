import type { ColumnDef, IndexDef, TableDef } from './types.js'

export type TableBuilderInput = {
  columns: ColumnDef[]
  indexes?: IndexDef[]
}

export function table(logicalName: string, tableName: string, input: TableBuilderInput): TableDef {
  if (input.columns.length === 0) {
    throw new Error(`schema table "${logicalName}" requires at least one column`)
  }
  const primaryKeys = input.columns.filter((c) => c.primaryKey)
  if (primaryKeys.length === 0) {
    throw new Error(`schema table "${logicalName}" requires a primary key column`)
  }
  return {
    logicalName,
    tableName,
    columns: input.columns,
    indexes: input.indexes ?? [],
  }
}

export function index(name: string, columns: string[], unique = false): IndexDef {
  return { name, columns, unique }
}

export function tenantPropertyIndex(): IndexDef {
  return index('tenant_property', ['tenantId', 'propertyId'])
}
