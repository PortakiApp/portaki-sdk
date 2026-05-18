import type { ModuleSchemaDef } from '../schema/types.js'
import { createModuleDb, type ModuleDb } from './table-query.js'
import type { ModuleDatabase } from './types.js'

export type RecordedDbCall = {
  readonly kind: 'query' | 'queryOne' | 'execute'
  readonly sql: string
  readonly params: readonly unknown[]
}

/**
 * Captures parameterized SQL emitted by {@link ModuleDb} (build-time codegen for the Java runtime).
 */
export class RecordingModuleDatabase implements ModuleDatabase {
  readonly calls: RecordedDbCall[] = []

  private readonly queryOneSeeds: (Record<string, unknown> | null)[] = []

  /** Queues return values for successive {@link queryOne} calls (e.g. insert vs update branches at build time). */
  seedQueryOneResponses(...rows: (Record<string, unknown> | null)[]): void {
    this.queryOneSeeds.push(...rows)
  }

  query<T extends Record<string, unknown> = Record<string, unknown>>(
    sql: string,
    params?: readonly unknown[],
  ): Promise<T[]> {
    this.calls.push({ kind: 'query', sql, params: params ?? [] })
    return Promise.resolve([])
  }

  queryOne<T extends Record<string, unknown> = Record<string, unknown>>(
    sql: string,
    params?: readonly unknown[],
  ): Promise<T | null> {
    this.calls.push({ kind: 'queryOne', sql, params: params ?? [] })
    if (this.queryOneSeeds.length > 0) {
      const seeded = this.queryOneSeeds.shift()!
      return Promise.resolve(seeded as T | null)
    }
    return Promise.resolve(null)
  }

  execute(sql: string, params?: readonly unknown[]): Promise<number> {
    this.calls.push({ kind: 'execute', sql, params: params ?? [] })
    return Promise.resolve(0)
  }
}

export function createRecordingModuleDb(schema: ModuleSchemaDef): {
  db: ModuleDb
  database: RecordingModuleDatabase
} {
  const database = new RecordingModuleDatabase()
  return { db: createModuleDb(schema, database), database }
}
