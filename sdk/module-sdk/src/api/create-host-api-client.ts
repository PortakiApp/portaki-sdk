import type {
  HostPropertyModuleItem,
  HostPropertyStatsPeriod,
  HostPropertyStatsResponse,
  HostModuleSyncResponse,
} from './host-types'

export type CreateHostApiClientOptions = {
  /**
   * URL de base de l’API hôte (ex. `https://api.portaki.app` ou vide pour même origine).
   * Ne doit pas se terminer par `/`.
   */
  baseUrl?: string
  /** Bearer JWT (hôte). */
  getAccessToken: () => string | Promise<string>
  /** Préfixe chemins (défaut `/api/v1`). */
  apiPrefix?: string
}

function joinUrl(base: string, path: string): string {
  if (!base) return path
  return `${base.replace(/\/$/, '')}${path.startsWith('/') ? '' : '/'}${path.replace(/^\//, '')}`
}

export function createHostApiClient(opts: CreateHostApiClientOptions) {
  const apiPrefix = opts.apiPrefix ?? '/api/v1'
  const base = opts.baseUrl ?? ''

  async function request<T>(
    path: string,
    init?: RequestInit & { json?: unknown },
  ): Promise<T> {
    const token = await opts.getAccessToken()
    const { json: jsonBody, headers: initHeaders, body: initBody, ...restInit } = init ?? {}
    const headers = new Headers(initHeaders as HeadersInit | undefined)
    headers.set('Authorization', `Bearer ${token}`)
    let body: BodyInit | undefined = initBody as BodyInit | undefined
    if (jsonBody !== undefined) {
      headers.set('Content-Type', 'application/json')
      body = JSON.stringify(jsonBody)
    }
    const url = joinUrl(base, `${apiPrefix}${path.startsWith('/') ? path : `/${path}`}`)
    const res = await fetch(url, {
      ...restInit,
      headers,
      body,
    })
    if (!res.ok) {
      const text = await res.text().catch(() => '')
      const err = new Error(`host_api_${res.status}: ${text.slice(0, 200)}`) as Error & { status: number }
      err.status = res.status
      throw err
    }
    if (res.status === 204) {
      return undefined as T
    }
    const ct = res.headers.get('content-type') ?? ''
    if (!ct.includes('application/json')) {
      return undefined as T
    }
    return (await res.json()) as T
  }

  return {
    /**
     * Appel HTTP authentifié relatif à `apiPrefix` (chemins type `/properties/...`).
     * Point d’extension pour le shell hôte sans exposer chaque route dans le SDK.
     */
    request,

    getPropertyStats(propertyId: string, period: HostPropertyStatsPeriod | string = '30d') {
      const q = new URLSearchParams({ period: String(period) })
      return request<HostPropertyStatsResponse>(
        `/properties/${encodeURIComponent(propertyId)}/stats?${q.toString()}`,
      )
    },

    listPropertyModules(propertyId: string) {
      return request<HostPropertyModuleItem[]>(`/properties/${encodeURIComponent(propertyId)}/modules`)
    },

    /** Lance l’action hôte `sync` pour un module officiel (`moduleId`, ex. `ical-sync`). */
    syncHostModule(propertyId: string, moduleId: string) {
      return request<HostModuleSyncResponse>(
        `/properties/${encodeURIComponent(propertyId)}/modules/${encodeURIComponent(moduleId)}/sync`,
        { method: 'POST' },
      )
    },
  }
}

export type PortakiHostApiClient = ReturnType<typeof createHostApiClient>
