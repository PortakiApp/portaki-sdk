'use client'

import { createContext, type ReactNode, useContext } from 'react'
import type { PortakiContext } from '../types'

export const PortakiContextInternal = createContext<PortakiContext | null>(null)

export type PortakiProviderValue = PortakiContext & {
  /** Base64url-encoded raw HMAC key material (server-issued, stay-scoped). */
  hmacKeyMaterialB64: string
}

const PortakiRuntimeContext = createContext<PortakiProviderValue | null>(null)

export function PortakiRuntimeProvider({
  value,
  children,
}: {
  value: PortakiProviderValue
  children: ReactNode
}) {
  const { hmacKeyMaterialB64: _k, ...ctx } = value
  return (
    <PortakiRuntimeContext.Provider value={value}>
      <PortakiContextInternal.Provider value={ctx}>{children}</PortakiContextInternal.Provider>
    </PortakiRuntimeContext.Provider>
  )
}

export function usePortakiRuntime(): PortakiProviderValue {
  const v = useContext(PortakiRuntimeContext)
  if (!v) {
    throw new Error('PortakiRuntimeProvider is missing')
  }
  return v
}
