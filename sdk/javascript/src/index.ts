export type { LangCode } from './types/module'
export type {
  StayData,
  PropertyData,
  TrackingEvent,
  ModuleContext,
  MapMarker,
  PortakiModuleDefinition,
  PortakiModuleDefinitionInput,
  NavSlot,
  StayStatus,
} from './types/module'
export { definePortakiModule } from './types/module'

export type {
  ModuleConfigFieldType,
  ModuleConfigAlert as ModuleConfigAlertSpec,
  ModuleConfigField,
  ModuleConfigSchema,
} from './types/config'

export type { PortakiGuestProperty, PortakiGuestStay, PortakiRenderContext } from './types/legacy'

export { useTracking } from './hooks/useTracking'
export type { UseTrackingOptions } from './hooks/useTracking'

export {
  ModuleSection,
  ModuleCard,
  ModuleLoading,
  ModuleError,
  ModuleEmpty,
  CopyButton,
  ExternalLink,
  WazeButton,
  GoogleMapsButton,
  ModuleConfigAlert,
} from './components'

/** Chargement dynamique du module par défaut (`definePortakiModule`). */
export type PortakiModuleLoader = () => Promise<{
  default: import('./types/module').PortakiModuleDefinition
}>
