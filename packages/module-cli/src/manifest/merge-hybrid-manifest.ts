import type { PortakiFullModule } from '@portaki/sdk/author'

import { gatewayManifestSlice } from '../codegen/manifest-backend.js'

export type HybridManifestMergeResult = {
  readonly merged: Record<string, unknown>
  readonly changed: boolean
}

/**
 * Hybrid manifest: keep catalogue/marketing fields; refresh gateway fields from defineModule + build.
 */
export function mergeHybridManifest(
  existing: Record<string, unknown>,
  module: PortakiFullModule,
): HybridManifestMergeResult {
  if (existing.id != null && existing.id !== module.id) {
    throw new Error(`portaki.module.json id "${existing.id}" does not match module "${module.id}"`)
  }

  const slice = gatewayManifestSlice(module)
  const previousScopes = Array.isArray(existing.scopes) ? [...(existing.scopes as string[])] : []
  const mergedScopes = [...new Set([...previousScopes, ...slice.scopes])].sort()

  const merged: Record<string, unknown> = {
    ...existing,
    id: module.id,
    version: module.version,
    database: slice.database ?? existing.database,
    queries: slice.queries ?? existing.queries,
    commands: slice.commands ?? existing.commands,
    scopes: mergedScopes,
  }

  if (module.backend) {
    merged.runtime = {
      ...(typeof existing.runtime === 'object' && existing.runtime != null
        ? (existing.runtime as Record<string, unknown>)
        : {}),
      backend: 'wasm',
    }
  }

  const changed = JSON.stringify(existing) !== JSON.stringify(merged)
  return { merged, changed }
}
