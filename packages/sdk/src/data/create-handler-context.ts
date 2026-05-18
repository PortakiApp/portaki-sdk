import { createModuleDb } from './table-query.js'
import type { HandlerContext, ModuleDatabase } from './types.js'
import type { ModuleSchemaDef } from '../schema/types.js'

export type HandlerContextInput = {
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

export type HandlerContextInputWithPublish = HandlerContextInput & {
  readonly onPublish?: (event: import('./types.js').ModulePublishedEvent) => void
}

export function createHandlerContext(input: HandlerContextInputWithPublish): HandlerContext {
  const onPublish = input.onPublish
  return {
    moduleId: input.moduleId,
    moduleVersion: input.moduleVersion,
    tenantId: input.tenantId,
    propertyId: input.propertyId,
    stayId: input.stayId,
    scopes: input.scopes,
    config: input.config,
    db: createModuleDb(input.schema, input.database),
    publish(eventName: string, payload: Record<string, unknown>) {
      onPublish?.({ name: eventName, payload })
    },
  }
}
