import type { ReactNode } from 'react'

import type { CommandDefinition, ModuleDataDefinition, QueryDefinition } from './data/types.js'
import { definePortakiModule } from './types/module.js'
import type {
  HostModuleContext,
  ModuleContext,
  PortakiModuleDefinition,
  PortakiModuleDefinitionInput,
} from './types/module.js'
import type { ModuleSchemaDef } from './schema/types.js'

export type PortakiFullModuleInput = PortakiModuleDefinitionInput & {
  readonly schema?: ModuleSchemaDef
  /** Semver stored in manifest `database.schemaVersion` (default: module `version`). */
  readonly schemaVersion?: string
  readonly queries?: Readonly<Record<string, QueryDefinition>>
  readonly commands?: Readonly<Record<string, CommandDefinition>>
}

export type PortakiFullModule = PortakiModuleDefinition & {
  readonly data?: ModuleDataDefinition
}

/**
 * Define a Portaki module (UI + optional schema, queries, commands).
 * Run `portaki build` to emit manifest and bundles (no hand-written SQL in repo).
 */
export function defineModule(input: PortakiFullModuleInput): PortakiFullModule {
  const guest = definePortakiModule(input)
  const hasData =
    input.schema != null &&
    (Object.keys(input.queries ?? {}).length > 0 || Object.keys(input.commands ?? {}).length > 0)

  if (input.schema == null && (input.queries != null || input.commands != null)) {
    throw new Error('defineModule: `schema` is required when declaring queries or commands')
  }

  if (!hasData) {
    return guest
  }

  const schemaVersion = input.schemaVersion ?? guest.version
  const schema = input.schema!
  const data: ModuleDataDefinition = {
    schema,
    schemaVersion,
    queries: input.queries ?? {},
    commands: input.commands ?? {},
  }

  return { ...guest, data }
}

export type DefineModuleGuestOnly = {
  render: (ctx: ModuleContext) => ReactNode
  renderHost?: (ctx: HostModuleContext) => ReactNode
}
