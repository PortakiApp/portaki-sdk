import type { ReactNode } from 'react'

export type LangCode = 'fr' | 'en'

export interface PortakiGuestProperty {
  id: string
  trainStationCode?: string
  checklistItems?: readonly { id: string; labelFr: string; labelEn: string }[]
}

export interface PortakiGuestStay {
  id: string
}

export interface PortakiRenderContext {
  property: PortakiGuestProperty
  stay?: PortakiGuestStay
  lang: LangCode
}

export interface PortakiModuleDefinition {
  id: string
  label: Record<string, string>
  icon: string
  navSlot?: 'section' | 'bottom-bar' | 'poi-overlay'
  mapOverlay?: boolean
  visibleOnStatus?: readonly string[]
  render: (ctx: PortakiRenderContext) => ReactNode
  mapMarkers?: (ctx: PortakiRenderContext) => Promise<unknown[]>
}

export function definePortakiModule(def: PortakiModuleDefinition): PortakiModuleDefinition {
  return def
}

/** Chargement dynamique du module par défaut (`definePortakiModule`). */
export type PortakiModuleLoader = () => Promise<{ default: PortakiModuleDefinition }>
