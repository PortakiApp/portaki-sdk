/** Période acceptée par `GET /api/v1/properties/:id/stats?period=` (suffixe `d` ou `h`). */
export type HostPropertyStatsPeriod = `${number}d` | `${number}h`

export type HostModuleCountDto = {
  id: string
  count: number
}

export type HostPropertyNextStayDto = {
  id: string
  guestName: string
  checkinAt: string
  checkoutAt: string
} | null

/** Réponse `GET /api/v1/properties/:id/stats` */
export type HostPropertyStatsResponse = {
  totalStays: number
  uniqueGuestsWithEmail: number
  staysWithoutEmail: number
  occupancyRate30d: number
  occupancyRate90d: number
  totalEvents: number
  avgSessionMinutes: number
  checklistCompletionRate: number
  topModules: HostModuleCountDto[]
  leastViewedModuleId: string | null
  eventsByHourUtc: number[]
  moduleViewCounts: HostModuleCountDto[]
  nextStay: HostPropertyNextStayDto
}

/** Élément `GET /api/v1/properties/:id/modules` */
export type HostPropertyModuleItem = {
  moduleId: string
  label: { fr: string; en: string }
  description: { fr: string; en: string }
  icon: string
  version: string
  tenantEnabled: boolean
  active: boolean
  config: Record<string, unknown>
  incomplete: boolean
  requiresConfig: boolean
  audience?: string
}

/** Réponse `POST /api/v1/properties/:id/modules/:moduleId/sync` (action hôte générique, sans vocabulaire métier module). */
export type HostModuleSyncResponse = {
  ok: boolean
  succeeded: number
  failed: number
  itemsTotal: number
  summary: string
}
