import '@testing-library/jest-dom/vitest'
import * as React from 'react'
import { afterEach, vi } from 'vitest'

;(globalThis as typeof globalThis & { React: typeof React }).React = React

afterEach(() => {
  vi.unstubAllGlobals()
})

Object.assign(navigator, {
  clipboard: {
    writeText: vi.fn().mockResolvedValue(undefined),
  },
})
