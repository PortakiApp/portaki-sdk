export type { LangCode } from './types/module'
export type {
  StayData,
  PropertyData,
  TrackingEvent,
  ModuleContext,
  HostModuleContext,
  MapMarker,
  PortakiModuleDefinition,
  PortakiModuleDefinitionInput,
  NavSlot,
  StayStatus,
  ModuleSurface,
} from './types/module'
export { definePortakiModule } from './types/module'

export { portakiModule, guestModule, hostModule, PortakiModuleBuilder } from './builder'

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
