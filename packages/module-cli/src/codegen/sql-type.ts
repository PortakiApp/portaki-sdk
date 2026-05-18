import type { ColumnDef, ColumnType } from '@portaki/sdk/schema'

export function sqlType(column: ColumnDef): string {
  const parts: string[] = [baseType(column.type)]
  if (column.primaryKey) {
    parts.push('PRIMARY KEY')
  }
  if (!column.primaryKey && !column.nullable) {
    parts.push('NOT NULL')
  }
  if (column.unique) {
    parts.push('UNIQUE')
  }
  if (column.defaultSql) {
    parts.push(`DEFAULT ${column.defaultSql}`)
  }
  return parts.join(' ')
}

function baseType(type: ColumnType): string {
  switch (type) {
    case 'uuid':
      return 'UUID'
    case 'text':
      return 'TEXT'
    case 'int':
      return 'INT'
    case 'boolean':
      return 'BOOLEAN'
    case 'jsonb':
      return 'JSONB'
    case 'timestamptz':
      return 'TIMESTAMPTZ'
    default:
      return 'TEXT'
  }
}
