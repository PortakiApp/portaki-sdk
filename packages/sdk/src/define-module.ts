import type { ReactNode } from 'react'

import type { CommandDefinition, ModuleDataDefinition, QueryDefinition } from './data/types.js'
import type { ModuleExtensionDefinition } from './extension/types.js'
import { definePortakiModule } from './types/module.js'
import type {
  HostModuleContext,
  ModuleContext,
  PortakiModuleDefinition,
  PortakiModuleDefinitionInput,
} from './types/module.js'
import type { ModuleSchemaDef } from './schema/types.js'

export type PortakiFullModuleInput = PortakiModuleDefinitionInput &
  ModuleExtensionDefinition & {
    readonly schema?: ModuleSchemaDef
    /** Semver stored in manifest `database.schemaVersion` (default: module `version`). */
    readonly schemaVersion?: string
    readonly queries?: Readonly<Record<string, QueryDefinition>>
    readonly commands?: Readonly<Record<string, CommandDefinition>>
  }

export type PortakiFullModule = PortakiModuleDefinition &
  ModuleExtensionDefinition & {
    readonly data?: ModuleDataDefinition
  }

/**
 * Define a Portaki module (UI + optional schema, queries, commands).
 * Run `portaki build` to emit manifest and bundles (no hand-written SQL in repo).
 */
export function defineModule(input: PortakiFullModuleInput): PortakiFullModule {
  const guest = definePortakiModule(input)
  const extension: ModuleExtensionDefinition = {
    ...(input.hostActions != null && Object.keys(input.hostActions).length > 0
      ? { hostActions: input.hostActions }
      : {}),
    ...(input.eventHandlers != null && Object.keys(input.eventHandlers).length > 0
      ? { eventHandlers: input.eventHandlers }
      : {}),
  }
  const hasDataHandlers =
    Object.keys(input.queries ?? {}).length > 0 || Object.keys(input.commands ?? {}).length > 0

  if (!hasDataHandlers) {
    return { ...guest, ...extension }
  }

  const schemaVersion = input.schemaVersion ?? guest.version
  const data: ModuleDataDefinition = {
    schemaVersion,
    queries: input.queries ?? {},
    commands: input.commands ?? {},
  }

  if (input.schema != null) {
    return { ...guest, ...extension, data: { ...data, schema: input.schema } }
  }

  return { ...guest, ...extension, data }
}

export type DefineModuleGuestOnly = {
  render: (ctx: ModuleContext) => ReactNode
  renderHost?: (ctx: HostModuleContext) => ReactNode
}
