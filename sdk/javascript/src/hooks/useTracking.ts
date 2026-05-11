'use client'

import { useCallback } from 'react'

import type { TrackingEvent } from '../types/module'

export interface UseTrackingOptions {
  stayId: string
  moduleId: string
  tenantSlug: string
  accessCode: string
}

export function useTracking({
  stayId,
  moduleId,
  tenantSlug,
  accessCode,
}: UseTrackingOptions) {
  const track = useCallback(
    (event: TrackingEvent) => {
      void fetch(`/api/v1/guest/${encodeURIComponent(tenantSlug)}/${encodeURIComponent(accessCode)}/track`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          eventType: `module_${event.type}`,
          moduleId,
          metadata: { label: event.label, stayId, ...event.metadata },
        }),
        keepalive: true,
      }).catch(() => {})
    },
    [stayId, moduleId, tenantSlug, accessCode],
  )

  return { track }
}
