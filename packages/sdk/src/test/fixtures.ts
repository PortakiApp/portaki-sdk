import type { PortakiContext } from '../runtime/types/index.js'
import { deriveModuleHmacKeyMaterialB64 } from '../server/derive-key.js'

export const TEST_HMAC_SECRET = 'portaki-test-hmac-secret'

export function createTestPortakiContext(
  overrides: Partial<PortakiContext> & { moduleId?: string; stayId?: string } = {},
): PortakiContext {
  const moduleId = overrides.moduleId ?? 'rules'
  const stayId = overrides.stayId ?? 'stay-test-001'
  const { moduleId: _m, stayId: _s, ...rest } = overrides
  return {
    stay: {
      id: stayId,
      guestName: 'Marie Dupont',
      checkinAt: '2026-06-01T15:00:00.000Z',
      checkoutAt: '2026-06-08T10:00:00.000Z',
      checkinTime: '15:00',
      checkoutTime: '10:00',
      status: 'ACTIVE',
      lang: 'fr',
    },
    property: {
      id: 'property-test-001',
      name: 'Villa Test',
      address: '1 rue de la Plage',
      lat: 43.25,
      lng: 5.38,
      theme: { primaryHex: '#E8724A' },
    },
    lang: 'fr',
    config: {},
    scopes: ['stay:read', 'property:read'],
    moduleId,
    isPreview: false,
    ...rest,
  }
}

export function createTestHmacKeyMaterial(moduleId: string, stayId: string): string {
  return deriveModuleHmacKeyMaterialB64(TEST_HMAC_SECRET, moduleId, stayId)
}
