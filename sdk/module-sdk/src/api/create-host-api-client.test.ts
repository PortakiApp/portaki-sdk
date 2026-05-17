import { afterEach, describe, expect, it, vi } from 'vitest'

import { createHostApiClient } from './create-host-api-client'

describe('createHostApiClient', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('sends bearer token and parses JSON', async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      new Response(JSON.stringify({ ok: true }), {
        status: 200,
        headers: { 'content-type': 'application/json' },
      }),
    )
    vi.stubGlobal('fetch', fetchMock)

    const client = createHostApiClient({
      baseUrl: 'https://api.test',
      getAccessToken: () => 'token-abc',
    })

    const data = await client.request<{ ok: boolean }>('/properties/p1/modules')

    expect(data.ok).toBe(true)
    expect(fetchMock).toHaveBeenCalledOnce()
    const [, init] = fetchMock.mock.calls[0] as [string, RequestInit]
    expect((init.headers as Headers).get('Authorization')).toBe('Bearer token-abc')
  })

  it('throws with status on HTTP error', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue(new Response('nope', { status: 403 })),
    )

    const client = createHostApiClient({ getAccessToken: async () => 't' })

    await expect(client.listPropertyModules('p1')).rejects.toMatchObject({ status: 403 })
  })
})
