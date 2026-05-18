import type { ColumnDef, ColumnType } from './types.js'

type ColumnOptions = {
  nullable?: boolean
  primaryKey?: boolean
  unique?: boolean
  defaultSql?: string
}

function column(name: string, type: ColumnType, options: ColumnOptions = {}): ColumnDef {
  const sqlName = toSnakeCase(name)
  return {
    name,
    sqlName,
    type,
    nullable: options.nullable ?? false,
    primaryKey: options.primaryKey ?? false,
    unique: options.unique ?? false,
    defaultSql: options.defaultSql,
  }
}

function toSnakeCase(value: string): string {
  return value
    .replace(/([A-Z])/g, '_$1')
    .toLowerCase()
    .replace(/^_/, '')
}

export function uuid(name: string, options: ColumnOptions = {}): ColumnDef {
  return column(name, 'uuid', options)
}

export function text(name: string, options: ColumnOptions = {}): ColumnDef {
  return column(name, 'text', options)
}

export function int(name: string, options: ColumnOptions = {}): ColumnDef {
  return column(name, 'int', options)
}

export function boolean(name: string, options: ColumnOptions = {}): ColumnDef {
  return column(name, 'boolean', options)
}

export function jsonb(name: string, options: ColumnOptions = {}): ColumnDef {
  return column(name, 'jsonb', options)
}

export function timestamptz(name: string, options: ColumnOptions = {}): ColumnDef {
  return column(name, 'timestamptz', options)
}

export function uuidPrimaryKey(name = 'id'): ColumnDef {
  return uuid(name, {
    primaryKey: true,
    defaultSql: 'gen_random_uuid()',
  })
}

export function propertyId(): ColumnDef {
  return uuid('propertyId', { unique: true })
}

export function tenantId(): ColumnDef {
  return uuid('tenantId')
}
