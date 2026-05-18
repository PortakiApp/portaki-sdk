/**
 * @file extension/types.ts
 * @brief Node extension contracts — host actions and platform event handlers.
 *
 * @details
 * Handlers declared here are bundled into `extension.cjs` by `@portaki/cli` and executed
 * by `portaki-module-runtime` via Node (host property sync, outbound webhooks, etc.).
 * They must not import React or perform persistence outside the returned config payload.
 *
 * @copyright Portaki — SPDX-License-Identifier: MIT
 * @addtogroup module_extensions Module extensions (Node)
 * @{
 */

/**
 * @brief Execution context for host-only actions (`sync`, …).
 */
export type HostExtensionContext = {
  /** @brief Tenant UUID (string). */
  readonly tenantId: string
  /** @brief Property UUID (string). */
  readonly propertyId: string
  /** @brief Catalogue module id (e.g. `ical-sync`). */
  readonly moduleId: string
}

/**
 * @brief Result of a host action — mirrors platform `HostModuleRunOutcome`.
 */
export type HostModuleRunResult = {
  /** @brief Whether the run completed without fatal error. */
  readonly ok: boolean
  /** @brief Count of successful sub-steps (e.g. feeds fetched). */
  readonly succeeded: number
  /** @brief Count of failed sub-steps. */
  readonly failed: number
  /** @brief Aggregate metric (e.g. parsed events). */
  readonly itemsTotal: number
  /** @brief Human-readable log for host UI (`sync_summary` field). */
  readonly summary: string
  /**
   * @brief Updated module config JSON (plaintext secrets).
   * @remarks Platform encrypts secret fields before persistence.
   */
  readonly updatedPlainConfigJson: string
}

/**
 * @brief Host property action handler (e.g. `sync` on calendar feeds).
 *
 * @param ctx Tenant/property/module identifiers.
 * @param plainConfig Decrypted module configuration object.
 */
export type HostActionHandler = (
  ctx: HostExtensionContext,
  plainConfig: Record<string, unknown>,
) => Promise<HostModuleRunResult> | HostModuleRunResult

/**
 * @brief Context passed to subscribed platform event handlers.
 */
export type EventHandlerContext = {
  readonly tenantId: string
  readonly propertyId: string
  readonly moduleId: string
  /** @brief Stay id when the event is stay-scoped; otherwise `null`. */
  readonly stayId: string | null
  /** @brief Decrypted module configuration. */
  readonly config: Record<string, unknown>
}

/**
 * @brief Handler for manifest `events.subscribed[]` entries.
 *
 * @param ctx Property-scoped context.
 * @param eventData Platform event payload (shape depends on `eventName`).
 */
export type EventHandler = (
  ctx: EventHandlerContext,
  eventData: Record<string, unknown>,
) => Promise<void> | void

/**
 * @brief Optional extension segment attached to {@link defineModule}.
 */
export type ModuleExtensionDefinition = {
  /** @brief Map of action id → handler (manifest host sync uses action `sync`). */
  readonly hostActions?: Readonly<Record<string, HostActionHandler>>
  /** @brief Map of platform event name → handler. */
  readonly eventHandlers?: Readonly<Record<string, EventHandler>>
}

/** @} */
