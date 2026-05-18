export type PortakiOpenModuleEmailAction = {
  kind: 'open-module'
  moduleId: string
  actionId: string
}

export type PortakiEmailAction = PortakiOpenModuleEmailAction

const DEFAULT_ACTION_ID = 'default'

/**
 * Decode `portakiAction` query param from a module transactional email CTA.
 * Format: `open-module:<moduleId>:<actionId>` (actionId optional → `default`).
 */
export function parsePortakiEmailAction(raw: string | null | undefined): PortakiEmailAction | null {
  if (raw == null || raw.trim() === '') {
    return null
  }
  const value = raw.trim()
  const parts = value.split(':')
  if (parts.length < 2 || parts[0] !== 'open-module') {
    return null
  }
  const moduleId = parts[1]?.trim()
  if (!moduleId) {
    return null
  }
  const actionId = (parts[2]?.trim() || DEFAULT_ACTION_ID)
  return { kind: 'open-module', moduleId, actionId }
}
