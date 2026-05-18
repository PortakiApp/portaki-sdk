import { createModuleDb } from './table-query.js'
import type { GatewayContext, ModuleDatabase } from './types.js'
import type { ModuleSchemaDef } from '../schema/types.js'

export type GatewayContextInput = {
  readonly moduleId: string
  readonly moduleVersion: string
  readonly tenantId: string
  readonly propertyId: string
  readonly stayId: string | null
  readonly scopes: readonly string[]
  readonly config: Record<string, unknown>
  readonly schema: ModuleSchemaDef
  readonly database: ModuleDatabase
}

export function createGatewayContext(input: GatewayContextInput): GatewayContext {
  return {
    moduleId: input.moduleId,
    moduleVersion: input.moduleVersion,
    tenantId: input.tenantId,
    propertyId: input.propertyId,
    stayId: input.stayId,
    scopes: input.scopes,
    config: input.config,
    db: createModuleDb(input.schema, input.database),
  }
}
