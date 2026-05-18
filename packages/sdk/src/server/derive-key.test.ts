import { describe, expect, it } from 'vitest'

import { deriveModuleHmacKeyMaterialB64 } from './derive-key.js'

describe('deriveModuleHmacKeyMaterialB64', () => {
  it('whenSameInputs_thenDeterministic', () => {
    const a = deriveModuleHmacKeyMaterialB64('secret', 'rules', 'stay-1')
    const b = deriveModuleHmacKeyMaterialB64('secret', 'rules', 'stay-1')
    expect(a).toBe(b)
  })

  it('whenStayChanges_thenDifferentKey', () => {
    const a = deriveModuleHmacKeyMaterialB64('secret', 'rules', 'stay-1')
    const b = deriveModuleHmacKeyMaterialB64('secret', 'rules', 'stay-2')
    expect(a).not.toBe(b)
  })
})
