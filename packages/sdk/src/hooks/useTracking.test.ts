/** @vitest-environment jsdom */
import { renderHook, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { useTracking } from './useTracking.js'

describe('useTracking', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('whenTrackCalled_thenPostsGuestEvent', async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(null, { status: 204 }))
    vi.stubGlobal('fetch', fetchMock)

    const { result } = renderHook(() =>
      useTracking({
        stayId: 'stay-1',
        moduleId: 'rules',
        tenantSlug: 'demo',
        accessCode: 'ABCD',
      }),
    )

    result.current.track({ type: 'click', label: 'cta-wifi' })

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalled()
    })
    const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit]
    expect(url).toContain('/api/v1/guest/demo/ABCD/track')
    expect(init.method).toBe('POST')
  })
})
