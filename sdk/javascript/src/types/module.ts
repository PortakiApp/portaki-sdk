import type { ReactNode } from 'react'

import type { ModuleConfigSchema } from './config'

export type LangCode = 'fr' | 'en'

export type NavSlot = 'section' | 'bottom-bar' | 'poi-overlay'

export type StayStatus = 'PRE_ARRIVAL' | 'UPCOMING' | 'ACTIVE' | 'COMPLETED'

export interface StayData {
  id: string
  guestName: string
  guestLang: string
  checkinAt: string
  checkoutAt: string
  accessCode: string
  status: string
}

export interface PropertyData {
  id: string
  trainStationCode?: string
  checklistItems?: readonly { id: string; labelFr: string; labelEn: string }[]
  name?: string
  slug?: string
  address?: string
  /** Coordonnées du bien (page invité) — utilisées p. ex. par le module météo. */
  lat?: number
  lng?: number
}

export interface TrackingEvent {
  type: 'click' | 'view' | 'action'
  label: string
  metadata?: Record<string, unknown>
}

export interface ModuleContext {
  stay: StayData
  property: PropertyData
  lang: LangCode
  config: Record<string, string | boolean | number>
  track: (event: TrackingEvent) => void
}

export interface MapMarker {
  id: string
  lat: number
  lng: number
  title?: string
}

export interface PortakiModuleDefinition {
  id: string
  label: { fr: string; en: string }
  description: { fr: string; en: string }
  icon: string
  version: string
  author?: string

  navSlot: NavSlot
  defaultNavLabel?: { fr: string; en: string }
  defaultNavIcon?: string

  visibleOnStatus?: StayStatus[]
  mapOverlay?: boolean

  config?: ModuleConfigSchema

  render: (ctx: ModuleContext) => ReactNode

  mapMarkers?: (ctx: Omit<ModuleContext, 'track'>) => Promise<MapMarker[]>
}

export type PortakiModuleDefinitionInput = Partial<
  Pick<PortakiModuleDefinition, 'description' | 'version' | 'navSlot'>
> &
  Pick<PortakiModuleDefinition, 'id' | 'label' | 'icon' | 'render'> &
  Omit<
    PortakiModuleDefinition,
    'id' | 'label' | 'icon' | 'render' | 'description' | 'version' | 'navSlot'
  >

export function definePortakiModule(def: PortakiModuleDefinitionInput): PortakiModuleDefinition {
  const description = def.description ?? { fr: '', en: '' }
  const version = def.version ?? '0.1.0'
  const navSlot = def.navSlot ?? 'section'
  return { ...def, description, version, navSlot }
}
