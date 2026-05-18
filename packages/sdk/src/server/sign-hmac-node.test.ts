import { describe, expect, it } from 'vitest'

import { deriveModuleHmacKeyMaterialB64 } from './derive-key.js'
import { signHmacTokenNode } from './sign-hmac-node.js'
import { verifyHmacToken } from './verify-hmac.js'

describe('signHmacTokenNode', () => {
  it('whenSigned_thenVerifiesWithVerifyHmacToken', () => {
    const key = deriveModuleHmacKeyMaterialB64('secret', 'rules', 'stay-1')
    const token = signHmacTokenNode(key, {
      moduleId: 'rules',
      commandName: 'rules.content.save',
      stayId: 'stay-1',
      timestamp: Date.now(),
    })

    expect(
      verifyHmacToken(
        token,
        { moduleId: 'rules', commandName: 'rules.content.save', stayId: 'stay-1' },
        key,
      ),
    ).toBe(true)
  })
})
