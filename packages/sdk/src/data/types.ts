/**
 * @file types.ts
 * @brief Gateway handler contracts — scopes, context, query/command definitions.
 *
 * @details
 * Query and command handlers receive {@link HandlerContext} with a schema-bound
 * {@link ModuleDb}. At build time the CLI records SQL steps into `operations.bundle.json`;
 * at runtime the platform executes those steps — handlers must not embed raw SQL strings.
 *
 * @copyright Portaki — SPDX-License-Identifier: MIT
 * @addtogroup module_gateway Gateway handlers
 * @{
 */

import type { ModuleDb } from './table-query.js'
import type { ModuleSchemaDef } from '../schema/types.js'

export type HandlerScope =
  | 'stay:read'
  | 'property:read'
  | 'host:property:read'
  | 'host:property:write'
  | string

export type ModulePublishedEvent = {
  readonly name: string
  readonly payload: Record<string, unknown>
}

/** Context passed to query/command handlers (`ctx.db`, tenant, property, …). */
export type HandlerContext = {
  readonly moduleId: string
  readonly moduleVersion: string
  readonly tenantId: string
  readonly propertyId: string
  readonly stayId: string | null
  readonly scopes: readonly string[]
  readonly config: Record<string, unknown>
  /** Schema-bound API — no raw SQL in module handlers. */
  readonly db: ModuleDb
  /** Emit a module gateway event (platform bus). No-op when not supported by the runtime. */
  publish(eventName: string, payload: Record<string, unknown>): void
}

/** Host-provided DB access (Wasm imports / local dev adapter). */
export type ModuleDatabase = {
  query<T extends Record<string, unknown> = Record<string, unknown>>(
    sql: string,
    params?: readonly unknown[],
  ): Promise<T[]>
  queryOne<T extends Record<string, unknown> = Record<string, unknown>>(
    sql: string,
    params?: readonly unknown[],
  ): Promise<T | null>
  execute(sql: string, params?: readonly unknown[]): Promise<number>
}

export type QueryHandler<TParams = Record<string, unknown>, TResult = unknown> = (
  ctx: HandlerContext,
  params: TParams,
) => Promise<TResult> | TResult

export type CommandHandler<TParams = Record<string, unknown>> = (
  ctx: HandlerContext,
  params: TParams,
) => Promise<void> | void

export type QueryDefinition<TParams = Record<string, unknown>, TResult = unknown> = {
  readonly scope: HandlerScope
  readonly description?: { fr: string; en: string }
  readonly handler: QueryHandler<TParams, TResult>
}

export type CommandDefinition<TParams = Record<string, unknown>> = {
  readonly scope: HandlerScope
  readonly description?: { fr: string; en: string }
  readonly handler: CommandHandler<TParams>
}

export type ModuleDataDefinition = {
  /** Omitted for config-only handlers (no `ctx.db` tables). */
  readonly schema?: ModuleSchemaDef
  readonly schemaVersion: string
  readonly queries: Readonly<Record<string, QueryDefinition>>
  readonly commands: Readonly<Record<string, CommandDefinition>>
}

/** @} */
