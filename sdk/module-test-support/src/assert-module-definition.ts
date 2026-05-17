import type { PortakiModuleDefinition } from '@portaki/module-sdk'

function assertModuleBase(def: PortakiModuleDefinition): void {
  if (!def.id || def.id.trim() === '') {
    throw new Error('module definition: id is required')
  }
  if (!def.label?.fr || !def.label?.en) {
    throw new Error(`module ${def.id}: label.fr and label.en are required`)
  }
  if (!def.icon) {
    throw new Error(`module ${def.id}: icon is required`)
  }
}

export function assertModuleDefinition(def: PortakiModuleDefinition): void {
  assertModuleBase(def)
  if (typeof def.render !== 'function' && typeof def.renderHost !== 'function') {
    throw new Error(`module ${def.id}: render and/or renderHost is required`)
  }
}

export function assertGuestSurface(def: PortakiModuleDefinition): void {
  assertModuleBase(def)
  if (def.surface === 'host') {
    throw new Error(`module ${def.id}: expected guest surface`)
  }
  if (typeof def.render !== 'function') {
    throw new Error(`module ${def.id}: render is required`)
  }
}

export function assertHostSurface(def: PortakiModuleDefinition): void {
  assertModuleBase(def)
  if (!def.renderHost) {
    throw new Error(`module ${def.id}: renderHost is required for host surface`)
  }
}
