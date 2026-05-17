import { describe, expect, it } from 'vitest'

import { guestModule, portakiModule } from './builder'
import { definePortakiModule } from './types/module'

describe('PortakiModuleBuilder', () => {
  it('builds guest module with defaults', () => {
    const def = guestModule('checklist')
      .label('Départ', 'Checkout')
      .icon('check-square')
      .guestRender(() => null)
      .build()

    expect(def.id).toBe('checklist')
    expect(def.navSlot).toBe('section')
    expect(def.surface).toBe('guest')
    expect(def.label.fr).toBe('Départ')
  })

  it('detects both surfaces when guest and host render are set', () => {
    const def = portakiModule('ical-sync')
      .guestRender(() => null)
      .hostRender(() => null)
      .build()

    expect(def.surface).toBe('both')
  })
})

describe('definePortakiModule', () => {
  it('throws when no render functions', () => {
    expect(() =>
      definePortakiModule({
        id: 'x',
        label: { fr: 'x', en: 'x' },
        icon: 'x',
      }),
    ).toThrow(/render/)
  })
})
