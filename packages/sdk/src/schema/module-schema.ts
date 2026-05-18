/**
 * @file module-schema.ts
 * @brief Schema DSL entry — declares module-owned PostgreSQL tables.
 *
 * @details
 * Tables defined here drive Flyway-compatible DDL generation (`migrations.bundle.json`)
 * and the typed `ctx.db` API inside gateway handlers. Table names must follow platform
 * conventions: `t_e_module_<moduleId>_<entity>`.
 *
 * @copyright Portaki — SPDX-License-Identifier: MIT
 * @addtogroup module_schema Schema DSL
 * @{
 */

import type { ModuleSchemaDef, TableDef } from './types.js'

/**
 * @brief Builds a module schema from table definitions.
 *
 * @param tables One or more tables created with {@link table}.
 * @returns Schema consumed by `defineModule({ schema })` and the CLI migrator.
 * @throws {Error} When `tables` is empty.
 *
 * @example
 * ```ts
 * const schema = moduleSchema([
 *   table('content', 't_e_module_rules_content', {
 *     columns: [uuidPrimaryKey(), propertyId(), tenantId(), jsonb('contentFr')],
 *     indexes: [index('tenant', ['tenantId'])],
 *   }),
 * ])
 * ```
 */
export function moduleSchema(tables: TableDef[]): ModuleSchemaDef {
  if (tables.length === 0) {
    throw new Error('moduleSchema requires at least one table')
  }
  return { tables }
}

/** @} */
