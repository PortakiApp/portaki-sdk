import type { ModuleSchemaDef, TableDef } from './types.js'

export function moduleSchema(tables: TableDef[]): ModuleSchemaDef {
  if (tables.length === 0) {
    throw new Error('moduleSchema requires at least one table')
  }
  return { tables }
}
