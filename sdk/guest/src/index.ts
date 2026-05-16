export { PortakiProvider, type PortakiProviderProps } from './context/PortakiProvider'
export {
  usePortakiCommand,
  usePortakiConfig,
  usePortakiContext,
  usePortakiModuleQuery,
  usePortakiQuery,
} from './hooks/portaki-hooks'
export { portaki, slotRegistry } from './slots/slot-registry'
export { parsePortakiEmailAction } from './email/parse-portaki-email-action'
export type { PortakiEmailAction, PortakiOpenModuleEmailAction } from './email/parse-portaki-email-action'
export type {
  DependencyQueryOptions,
  PortakiContext,
  PortakiError,
  PortakiLang,
  PropertyData,
  PropertyTheme,
  QueryResult,
  SlotDefinition,
  SlotName,
  StayData,
} from './types'
