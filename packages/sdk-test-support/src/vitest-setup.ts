import '@testing-library/jest-dom/vitest'
import * as React from 'react'
import { vi } from 'vitest'

;(globalThis as typeof globalThis & { React: typeof React }).React = React

vi.stubGlobal(
  'fetch',
  vi.fn().mockResolvedValue(
    new Response(JSON.stringify({ sections: [], items: [] }), {
      status: 200,
      headers: { 'content-type': 'application/json' },
    }),
  ),
)
