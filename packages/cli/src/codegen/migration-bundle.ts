import type { SchemaMigrationRevision } from './generate-schema-sql.js'

export type ModuleMigrationBundle = {
  readonly moduleId: string
  readonly schemaVersion: string
  readonly revisions: readonly {
    readonly revision: string
    readonly sql: string
  }[]
}

export function toMigrationBundle(
  moduleId: string,
  schemaVersion: string,
  revisions: SchemaMigrationRevision[],
): ModuleMigrationBundle {
  return {
    moduleId,
    schemaVersion,
    revisions: revisions.map((r) => ({ revision: r.revision, sql: r.sql })),
  }
}
