export type ModuleConfigFieldType =
  | 'text'
  | 'secret'
  | 'select'
  | 'toggle'
  | 'number'
  | 'url'
  | 'textarea'
  | 'readonly'

export interface ModuleConfigAlert {
  type: 'info' | 'warning' | 'error' | 'success'
  message: { fr: string; en: string }
  helpUrl?: string
}

export interface ModuleConfigField {
  key: string
  label: { fr: string; en: string }
  description?: { fr: string; en: string }
  type: ModuleConfigFieldType
  required: boolean
  default?: string | boolean | number
  placeholder?: { fr: string; en: string }
  options?: Array<{ value: string; label: { fr: string; en: string } }>
  min?: number
  max?: number
  /** Runtime-only validation for modules defined in TypeScript code */
  validation?: RegExp
  /** Serializable pattern for HTTP/registry-backed schemas */
  validationPattern?: string
  alert?: ModuleConfigAlert
}

export interface ModuleConfigSchema {
  fields: ModuleConfigField[]
  globalAlert?: ModuleConfigAlert
}
