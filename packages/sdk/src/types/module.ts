/**
 * @file module.ts
 * @brief Guest and host UI contracts — module manifest, contexts, and `definePortakiModule`.
 *
 * @details
 * Defines the presentation layer of a catalogue module: labels, navigation slot,
 * render functions, and optional map overlays. For gateway persistence use
 * {@link defineModule} which extends these types with `queries` / `commands`.
 *
 * @copyright Portaki — SPDX-License-Identifier: MIT
 * @addtogroup module_ui Module UI contracts
 * @{
 */

import type { ReactNode } from 'react'

import type { ModuleConfigSchema } from './config'

export type LangCode = 'fr' | 'en'

export type NavSlot = 'section' | 'bottom-bar' | 'poi-overlay' | 'post-stay'

export type StayStatus = 'PRE_ARRIVAL' | 'UPCOMING' | 'ACTIVE' | 'COMPLETED'

export type ModuleSurface = 'guest' | 'host' | 'both'

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

/** Contexte pour un rendu côté hôte (dashboard / réglages logement). */
export interface HostModuleContext {
  lang: LangCode
  propertyId: string
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

  /** Cible d’affichage : invité, hôte, ou les deux (dérivé si omis). */
  surface?: ModuleSurface

  render: (ctx: ModuleContext) => ReactNode

  renderHost?: (ctx: HostModuleContext) => ReactNode

  mapMarkers?: (ctx: Omit<ModuleContext, 'track'>) => Promise<MapMarker[]>
}

export type PortakiModuleDefinitionInput = Partial<
  Pick<
    PortakiModuleDefinition,
    | 'description'
    | 'version'
    | 'navSlot'
    | 'surface'
    | 'renderHost'
    | 'render'
    | 'mapMarkers'
    | 'author'
    | 'defaultNavLabel'
    | 'defaultNavIcon'
    | 'visibleOnStatus'
    | 'mapOverlay'
    | 'config'
  >
> &
  Pick<PortakiModuleDefinition, 'id' | 'label' | 'icon'> &
  Omit<
    PortakiModuleDefinition,
    | 'id'
    | 'label'
    | 'icon'
    | 'render'
    | 'renderHost'
    | 'description'
    | 'version'
    | 'navSlot'
    | 'surface'
    | 'mapMarkers'
    | 'author'
    | 'defaultNavLabel'
    | 'defaultNavIcon'
    | 'visibleOnStatus'
    | 'mapOverlay'
    | 'config'
  >

/**
 * @brief Defines UI surfaces for a Portaki module (guest livret and/or host workspace).
 *
 * @param def Catalogue metadata and React render functions.
 * @returns Normalized {@link PortakiModuleDefinition} with inferred `surface` when omitted.
 * @throws {Error} When neither `render` nor `renderHost` is provided.
 *
 * @remarks Prefer {@link defineModule} for new gateway modules so `portaki build` can emit bundles.
 */
export function definePortakiModule(def: PortakiModuleDefinitionInput): PortakiModuleDefinition {
  const description = def.description ?? { fr: '', en: '' }
  const version = def.version ?? '0.1.0'
  const navSlot = def.navSlot ?? 'section'
  if (!def.render && !def.renderHost) {
    throw new Error('@portaki/sdk: definePortakiModule requires `render` and/or `renderHost`')
  }
  const render = def.render ?? (() => null)
  let surface: ModuleSurface | undefined = def.surface
  if (!surface) {
    if (def.renderHost && def.render) {
      surface = 'both'
    } else if (def.renderHost) {
      surface = 'host'
    } else {
      surface = 'guest'
    }
  }
  return { ...def, description, version, navSlot, render, surface }
}

/** @} */
