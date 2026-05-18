import type { ReactNode } from 'react'

import type { BackendDefinition, CommandDefinition, QueryDefinition } from '../gateway/types.js'
import { definePortakiModule } from '../types/module.js'
import type {
  HostModuleContext,
  ModuleContext,
  PortakiModuleDefinition,
  PortakiModuleDefinitionInput,
} from '../types/module.js'
import type { ModuleSchemaDef } from '../schema/types.js'

export type PortakiFullModuleInput = PortakiModuleDefinitionInput & {
  readonly schema?: ModuleSchemaDef
  /** Semver stored in manifest `database.schemaVersion` (default: module `version`). */
  readonly schemaVersion?: string
  readonly queries?: Readonly<Record<string, QueryDefinition>>
  readonly commands?: Readonly<Record<string, CommandDefinition>>
}

export type PortakiFullModule = PortakiModuleDefinition & {
  readonly backend?: BackendDefinition
}

/**
 * Single entry point for module authors: guest UI + optional backend (schema, queries, commands).
 * Run `portaki-module build` to emit manifest, migration bundle, and backend Wasm (no hand-written SQL in repo).
 */
export function defineModule(input: PortakiFullModuleInput): PortakiFullModule {
  const guest = definePortakiModule(input)
  const hasBackend =
    input.schema != null &&
    (Object.keys(input.queries ?? {}).length > 0 || Object.keys(input.commands ?? {}).length > 0)

  if (input.schema == null && (input.queries != null || input.commands != null)) {
    throw new Error('defineModule: `schema` is required when declaring queries or commands')
  }

  if (!hasBackend) {
    return guest
  }

  const schemaVersion = input.schemaVersion ?? guest.version
  const backend: BackendDefinition = {
    schema: input.schema,
    schemaVersion,
    queries: input.queries ?? {},
    commands: input.commands ?? {},
  }

  return { ...guest, backend }
}

export type DefineModuleGuestOnly = {
  render: (ctx: ModuleContext) => ReactNode
  renderHost?: (ctx: HostModuleContext) => ReactNode
}
