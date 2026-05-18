import { describe, expect, it } from 'vitest'

import { definePortakiModule } from '@portaki/sdk'

import {
  assertGuestSurface,
  assertHostSurface,
  assertModuleDefinition,
} from './assert-module-definition.js'

const validGuest = definePortakiModule({
  id: 'guest-mod',
  label: { fr: 'Invité', en: 'Guest' },
  icon: 'user',
  render: () => null,
})

describe('assertModuleDefinition', () => {
  it('whenValidGuest_thenPasses', () => {
    expect(() => assertModuleDefinition(validGuest)).not.toThrow()
  })

  it('whenMissingLabel_thenThrows', () => {
    expect(() =>
      assertModuleDefinition({
        ...validGuest,
        label: { fr: '', en: 'Guest' },
      }),
    ).toThrow(/label/)
  })

  it('whenNoRender_thenThrows', () => {
    expect(() =>
      assertModuleDefinition({
        id: 'x',
        label: { fr: 'x', en: 'x' },
        icon: 'x',
      } as typeof validGuest),
    ).toThrow(/render/)
  })
})

describe('assertGuestSurface', () => {
  it('whenHostOnly_thenThrows', () => {
    const hostOnly = definePortakiModule({
      id: 'host-only',
      label: { fr: 'H', en: 'H' },
      icon: 'box',
      renderHost: () => null,
    })
    expect(() => assertGuestSurface(hostOnly)).toThrow(/guest surface/)
  })
})

describe('assertHostSurface', () => {
  it('whenMissingRenderHost_thenThrows', () => {
    expect(() => assertHostSurface(validGuest)).toThrow(/renderHost/)
  })
})
