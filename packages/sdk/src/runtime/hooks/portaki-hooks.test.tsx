import { renderHook, waitFor } from '@testing-library/react'
import type { ReactNode } from 'react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { PortakiProvider } from '../context/PortakiProvider.js'
import { createTestHmacKeyMaterial, createTestPortakiContext } from '../../test/fixtures.js'
import {
  usePortakiCommand,
  usePortakiConfig,
  usePortakiContext,
  usePortakiQuery,
} from './portaki-hooks.js'

describe('usePortakiContext', () => {
  it('whenOutsideProvider_thenThrows', () => {
    expect(() => renderHook(() => usePortakiContext())).toThrow(/PortakiProvider/)
  })
})

describe('usePortakiQuery', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('whenQuerySucceeds_thenReturnsData', async () => {
    const context = createTestPortakiContext({ moduleId: 'rules', stayId: 'stay-1' })
    const key = createTestHmacKeyMaterial('rules', 'stay-1')
    const fetchMock = vi.fn().mockResolvedValue(
      new Response(JSON.stringify({ contentFr: { type: 'doc' } }), {
        status: 200,
        headers: { 'content-type': 'application/json' },
      }),
    )
    vi.stubGlobal('fetch', fetchMock)

    const wrapper = ({ children }: { children: ReactNode }) => (
      <PortakiProvider context={context} hmacKeyMaterialB64={key}>
        {children}
      </PortakiProvider>
    )

    const { result } = renderHook(() => usePortakiQuery<Record<string, unknown>>('rules.content'), {
      wrapper,
    })

    await waitFor(() => {
      expect(result.current.loading).toBe(false)
    })
    expect(result.current.data).toEqual({ contentFr: { type: 'doc' } })
    expect(result.current.error).toBeNull()
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/portaki/query',
      expect.objectContaining({
        method: 'POST',
        headers: expect.objectContaining({
          'X-Portaki-Module': 'rules',
        }),
      }),
    )
  })

  it('whenCommandSucceeds_thenExecuteResolves', async () => {
    const context = createTestPortakiContext({ moduleId: 'rules', stayId: 'stay-1' })
    const key = createTestHmacKeyMaterial('rules', 'stay-1')
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue(new Response(null, { status: 204 })),
    )

    const wrapper = ({ children }: { children: ReactNode }) => (
      <PortakiProvider context={context} hmacKeyMaterialB64={key}>
        {children}
      </PortakiProvider>
    )

    const { result } = renderHook(() => usePortakiCommand('rules.content.save'), { wrapper })
    await result.current.execute({ contentFr: null })

    expect(result.current.error).toBeNull()
  })

  it('whenConfigPresent_thenUsePortakiConfigReturnsIt', () => {
    const context = createTestPortakiContext({
      config: { showWifi: true },
    })
    const wrapper = ({ children }: { children: ReactNode }) => (
      <PortakiProvider context={context} hmacKeyMaterialB64="dGVzdA==">
        {children}
      </PortakiProvider>
    )

    const { result } = renderHook(() => usePortakiConfig<{ showWifi: boolean }>(), { wrapper })
    expect(result.current.showWifi).toBe(true)
  })
})
