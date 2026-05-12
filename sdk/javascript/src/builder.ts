import type { ReactNode } from 'react'

import { definePortakiModule } from './types/module'
import type {
  HostModuleContext,
  ModuleContext,
  NavSlot,
  PortakiModuleDefinition,
  PortakiModuleDefinitionInput,
  StayStatus,
} from './types/module'
import type { ModuleConfigSchema } from './types/config'

type LocalePair = { fr: string; en: string }

type BuilderState = {
  id: string
  label?: LocalePair
  description?: LocalePair
  icon?: string
  version?: string
  author?: string
  navSlot?: NavSlot
  defaultNavLabel?: LocalePair
  defaultNavIcon?: string
  visibleOnStatus?: StayStatus[]
  mapOverlay?: boolean
  config?: ModuleConfigSchema
  render?: (ctx: ModuleContext) => ReactNode
  renderHost?: (ctx: HostModuleContext) => ReactNode
  mapMarkers?: PortakiModuleDefinition['mapMarkers']
}

/**
 * Builder fluent pour composer un module sans un seul gros objet littéral.
 * `guestModule` / `hostModule` sont des alias pour la lisibilité.
 */
export class PortakiModuleBuilder {
  private readonly state: BuilderState

  constructor(id: string) {
    this.state = { id }
  }

  label(fr: string, en: string) {
    this.state.label = { fr, en }
    return this
  }

  description(fr: string, en: string) {
    this.state.description = { fr, en }
    return this
  }

  icon(name: string) {
    this.state.icon = name
    return this
  }

  version(v: string) {
    this.state.version = v
    return this
  }

  author(author: string) {
    this.state.author = author
    return this
  }

  navSlot(slot: NavSlot) {
    this.state.navSlot = slot
    return this
  }

  defaultNavLabel(fr: string, en: string) {
    this.state.defaultNavLabel = { fr, en }
    return this
  }

  defaultNavIcon(icon: string) {
    this.state.defaultNavIcon = icon
    return this
  }

  visibleOnStatus(statuses: StayStatus[]) {
    this.state.visibleOnStatus = statuses
    return this
  }

  mapOverlay(v: boolean) {
    this.state.mapOverlay = v
    return this
  }

  schema(c: ModuleConfigSchema) {
    this.state.config = c
    return this
  }

  guestRender(render: (ctx: ModuleContext) => ReactNode) {
    this.state.render = render
    return this
  }

  hostRender(renderHost: (ctx: HostModuleContext) => ReactNode) {
    this.state.renderHost = renderHost
    return this
  }

  mapMarkers(f: PortakiModuleDefinition['mapMarkers']) {
    this.state.mapMarkers = f
    return this
  }

  build(): PortakiModuleDefinition {
    const label = this.state.label ?? { fr: this.state.id, en: this.state.id }
    const icon = this.state.icon ?? 'puzzle'
    const input: PortakiModuleDefinitionInput = {
      id: this.state.id,
      label,
      icon,
      description: this.state.description,
      version: this.state.version,
      author: this.state.author,
      navSlot: this.state.navSlot,
      defaultNavLabel: this.state.defaultNavLabel,
      defaultNavIcon: this.state.defaultNavIcon,
      visibleOnStatus: this.state.visibleOnStatus,
      mapOverlay: this.state.mapOverlay,
      config: this.state.config,
      render: this.state.render,
      renderHost: this.state.renderHost,
      mapMarkers: this.state.mapMarkers,
    }
    return definePortakiModule(input)
  }
}

export function portakiModule(id: string): PortakiModuleBuilder {
  return new PortakiModuleBuilder(id)
}

export const guestModule = portakiModule
export const hostModule = portakiModule
