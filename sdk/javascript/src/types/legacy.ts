import type { LangCode } from './module'

/**
 * @deprecated Prefer {@link ModuleContext} — kept for gradual migration of loaders.
 */
export interface PortakiGuestProperty {
  id: string
  trainStationCode?: string
  checklistItems?: readonly { id: string; labelFr: string; labelEn: string }[]
}

/**
 * @deprecated Prefer {@link ModuleContext}
 */
export interface PortakiGuestStay {
  id: string
}

/**
 * @deprecated Prefer {@link ModuleContext}
 */
export interface PortakiRenderContext {
  property: PortakiGuestProperty
  stay?: PortakiGuestStay
  lang: LangCode
}
