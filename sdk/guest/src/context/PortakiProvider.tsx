'use client'

import type { ReactNode } from 'react'
import type { PortakiContext } from '../types'
import { PortakiRuntimeProvider, type PortakiProviderValue } from './portaki-internal-context'

export type PortakiProviderProps = {
  context: PortakiContext
  /** Server-rendered per-session HMAC key (opaque to callers). */
  hmacKeyMaterialB64: string
  children: ReactNode
}

export function PortakiProvider({ context, hmacKeyMaterialB64, children }: PortakiProviderProps) {
  const value: PortakiProviderValue = { ...context, hmacKeyMaterialB64 }
  return <PortakiRuntimeProvider value={value}>{children}</PortakiRuntimeProvider>
}
