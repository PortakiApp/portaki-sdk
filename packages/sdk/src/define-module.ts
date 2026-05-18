/**
 * @file define-module.ts
 * @brief Primary module authoring API — composes UI, gateway handlers, and Node extensions.
 *
 * @details
 * `defineModule` is the single entry point module authors use in `src/portaki.module.ts`.
 * It merges guest/host UI (`definePortakiModule`) with optional data-plane definitions
 * (schema, queries, commands) and runtime extensions (`hostActions`, `eventHandlers`).
 *
 * Run `@portaki/cli` (`portaki build`) to emit:
 * - `migrations.bundle.json` — DDL for the module schema (when `schema` is set)
 * - `operations.bundle.json` — recorded SQL operations for gateway dispatch
 * - `extension.cjs` — Node host/event handlers (when extensions are declared)
 *
 * @copyright Portaki — SPDX-License-Identifier: MIT
 * @addtogroup module_authoring Module authoring
 * @{
 */

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

/**
 * @brief Input shape for {@link defineModule} — UI manifest fields plus optional gateway and extensions.
 */
export type PortakiFullModuleInput = PortakiModuleDefinitionInput &
  ModuleExtensionDefinition & {
    /** @brief Module-owned tables. Omitted for config-only gateway handlers (e.g. iCal import list). */
    readonly schema?: ModuleSchemaDef
    /**
     * @brief Semver persisted in catalogue manifest `database.schemaVersion`.
     * @remarks Defaults to module `version` when omitted.
     */
    readonly schemaVersion?: string
    /** @brief Read handlers exposed via platform gateway (`scope` per handler). */
    readonly queries?: Readonly<Record<string, QueryDefinition>>
    /** @brief Write handlers; may emit domain events through `ctx.publish`. */
    readonly commands?: Readonly<Record<string, CommandDefinition>>
  }

/**
 * @brief Resolved module definition returned by {@link defineModule} and consumed by the CLI.
 */
export type PortakiFullModule = PortakiModuleDefinition &
  ModuleExtensionDefinition & {
    /** @brief Present when the module declares queries and/or commands. */
    readonly data?: ModuleDataDefinition
  }

/**
 * @brief Defines a Portaki catalogue module (UI + optional schema, gateway, extensions).
 *
 * @param input Module metadata, render functions, and optional data/extension segments.
 * @returns A frozen logical module definition passed to `portaki build`.
 *
 * @remarks
 * - UI-only modules: omit `queries` / `commands` — only guest/host React surfaces are required.
 * - Gateway modules: handlers use `ctx.db` (schema-bound API); never embed raw SQL strings in source.
 * - Host sync / webhooks: implement `hostActions` / `eventHandlers`; CLI bundles `extension.cjs`.
 *
 * @example
 * ```ts
 * export default defineModule({
 *   id: 'rules',
 *   label: { fr: 'Règlement', en: 'House rules' },
 *   icon: 'scale',
 *   version: '1.0.0',
 *   schema: rulesSchema,
 *   queries: { 'rules.content': { scope: 'property:read', handler: async (ctx) => ({}) } },
 *   render: ({ lang }) => <RulesGuestView lang={lang} />,
 * })
 * ```
 *
 * @see {@link definePortakiModule} for UI-only modules
 * @see `examples/rules/src/portaki.module.ts` for the reference implementation
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

/**
 * @brief Minimal UI-only module shape (no gateway segment).
 * @deprecated Prefer explicit `defineModule` / `definePortakiModule` in new code.
 */
export type DefineModuleGuestOnly = {
  render: (ctx: ModuleContext) => ReactNode
  renderHost?: (ctx: HostModuleContext) => ReactNode
}

/** @} */
