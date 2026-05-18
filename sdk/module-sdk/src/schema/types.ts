/** Logical column types for module-owned tables (modules DB, no FK to API core). */

export type ColumnType =
  | 'uuid'
  | 'text'
  | 'int'
  | 'boolean'
  | 'jsonb'
  | 'timestamptz'

export type ColumnDef = {
  readonly name: string
  readonly sqlName: string
  readonly type: ColumnType
  readonly nullable: boolean
  readonly primaryKey: boolean
  readonly unique: boolean
  readonly defaultSql?: string
}

export type IndexDef = {
  readonly name: string
  readonly columns: readonly string[]
  readonly unique: boolean
}

export type TableDef = {
  readonly logicalName: string
  readonly tableName: string
  readonly columns: readonly ColumnDef[]
  readonly indexes: readonly IndexDef[]
}

export type ModuleSchemaDef = {
  readonly tables: readonly TableDef[]
}
