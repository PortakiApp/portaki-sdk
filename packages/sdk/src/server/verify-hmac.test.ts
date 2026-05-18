import { describe, expect, it } from 'vitest'

import { deriveModuleHmacKeyMaterialB64 } from './derive-key.js'
import { signHmacTokenNode } from './sign-hmac-node.js'
import { verifyHmacToken } from './verify-hmac.js'

describe('verifyHmacToken', () => {
  it('whenValidToken_thenReturnsTrue', () => {
    const key = deriveModuleHmacKeyMaterialB64('master', 'rules', 'stay-1')
    const token = signHmacTokenNode(key, {
      moduleId: 'rules',
      queryName: 'rules.content',
      stayId: 'stay-1',
      timestamp: Date.now(),
    })

    expect(
      verifyHmacToken(
        token,
        { moduleId: 'rules', queryName: 'rules.content', stayId: 'stay-1' },
        key,
      ),
    ).toBe(true)
  })

  it('whenWrongModule_thenReturnsFalse', () => {
    const key = deriveModuleHmacKeyMaterialB64('master', 'rules', 'stay-1')
    const token = signHmacTokenNode(key, {
      moduleId: 'rules',
      stayId: 'stay-1',
      timestamp: Date.now(),
    })

    expect(verifyHmacToken(token, { moduleId: 'other', stayId: 'stay-1' }, key)).toBe(false)
  })

  it('whenExpiredTimestamp_thenReturnsFalse', () => {
    const key = deriveModuleHmacKeyMaterialB64('master', 'rules', 'stay-1')
    const token = signHmacTokenNode(key, {
      moduleId: 'rules',
      stayId: 'stay-1',
      timestamp: Date.now() - 600_000,
    })

    expect(
      verifyHmacToken(token, { moduleId: 'rules', stayId: 'stay-1' }, key, 120_000),
    ).toBe(false)
  })

  it('whenMalformedToken_thenReturnsFalse', () => {
    expect(verifyHmacToken('bad', { moduleId: 'rules', stayId: 's' }, 'key')).toBe(false)
  })
})
