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

export { defineModule } from './define-module'
export type { PortakiFullModule, PortakiFullModuleInput } from './define-module'

export type { ColumnDef, ColumnType, IndexDef, ModuleSchemaDef, TableDef } from './schema/types'
export { moduleSchema } from './schema/module-schema'
export { index, table, tenantPropertyIndex } from './schema/table'
export {
  boolean,
  int,
  jsonb,
  propertyId,
  tenantId,
  text,
  timestamptz,
  uuid,
  uuidPrimaryKey,
} from './schema/columns'

export type {
  CommandDefinition,
  CommandHandler,
  HandlerContext,
  HandlerScope,
  ModuleDataDefinition,
  ModuleDatabase,
  QueryDefinition,
  QueryHandler,
} from './data/types'

export { portakiModule, guestModule, hostModule, PortakiModuleBuilder } from './builder'

export type {
  ModuleConfigFieldType,
  ModuleConfigAlert as ModuleConfigAlertSpec,
  ModuleConfigField,
  ModuleConfigSchema,
} from './types/config'

export type {
  ModuleEmailDeclaration,
  ModuleEmailSkipWhen,
  ModuleEmailTrigger,
  ModuleEmailTriggerType,
  ModuleGuestActionDeclaration,
  ModuleGuestActionKind,
  ModuleHostSurface,
  ModuleHostSurfaceType,
  ModuleManifestHostHints,
} from './types/manifest'

export type { PortakiGuestProperty, PortakiGuestStay, PortakiRenderContext } from './types/legacy'

export { useTracking } from './hooks/useTracking'
export type { UseTrackingOptions } from './hooks/useTracking'

export { createHostApiClient } from './api/create-host-api-client'
export type { CreateHostApiClientOptions, PortakiHostApiClient } from './api/create-host-api-client'
export type {
  HostModuleCountDto,
  HostPropertyModuleItem,
  HostPropertyNextStayDto,
  HostPropertyStatsPeriod,
  HostPropertyStatsResponse,
  HostModuleSyncResponse,
} from './api/host-types'

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

export {
  PortakiProvider,
  usePortakiCommand,
  usePortakiConfig,
  usePortakiContext,
  usePortakiModuleQuery,
  usePortakiQuery,
  portaki,
  slotRegistry,
  parsePortakiEmailAction,
} from './runtime/index'
export type {
  PortakiProviderProps,
  PortakiProviderValue,
  PortakiEmailAction,
  PortakiOpenModuleEmailAction,
  DependencyQueryOptions,
  PortakiContext,
  PortakiError,
  PortakiLang,
  PropertyTheme,
  QueryResult,
  SlotDefinition,
  SlotName,
} from './runtime/index'

/** Chargement dynamique du module par défaut (`definePortakiModule` ou `defineModule`). */
export type PortakiModuleLoader = () => Promise<{
  default: import('./types/module').PortakiModuleDefinition | import('./define-module').PortakiFullModule
}>
