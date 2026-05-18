'use client'

import type { ComponentType } from 'react'
import type { SlotDefinition } from '../types'

export type SlotRegistration = {
  moduleId: string
  slot: SlotDefinition
  component: ComponentType<Record<string, unknown>>
}

class SlotRegistry {
  private slots = new Map<string, SlotRegistration[]>()

  register(moduleId: string, slot: SlotDefinition, component: ComponentType<Record<string, unknown>>) {
    const key = slot.name
    const existing = this.slots.get(key) ?? []
    this.slots.set(key, [...existing, { moduleId, slot, component }])
  }

  getSlot(name: string): SlotRegistration[] {
    return this.slots.get(name) ?? []
  }
}

export const slotRegistry = new SlotRegistry()

export const portaki = {
  slot: (
    moduleId: string,
    slot: SlotDefinition,
    component: ComponentType<Record<string, unknown>>,
  ) => {
    slotRegistry.register(moduleId, slot, component)
  },
}
