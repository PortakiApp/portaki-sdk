import { decodeModuleHmacKeyMaterial, signHmacPayload } from '../runtime/security/hmac.js'
import { deriveModuleHmacKeyMaterialB64 } from './derive-key'
import { verifyHmacToken } from './verify-hmac'
import { signHmacTokenNode } from './sign-hmac-node'

export { verifyHmacToken, deriveModuleHmacKeyMaterialB64, signHmacTokenNode }

export async function portakiServerQuery<T>(
  queryName: string,
  params: Record<string, unknown>,
  context: { moduleId: string; stayId: string },
  opts: { hmacKeyMaterialB64: string; baseUrl: string },
): Promise<T> {
  const keyMaterial = decodeModuleHmacKeyMaterial(opts.hmacKeyMaterialB64)
  const token = await signHmacPayload(keyMaterial, {
    moduleId: context.moduleId,
    queryName,
    stayId: context.stayId,
    timestamp: Date.now(),
  })
  const base = opts.baseUrl.replace(/\/$/, '')
  const res = await fetch(`${base}/api/portaki/query`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'X-Portaki-Module': context.moduleId,
      'X-Portaki-Token': token,
    },
    body: JSON.stringify({ query: queryName, params: { ...params, _stayId: context.stayId } }),
    cache: 'no-store',
  })
  if (!res.ok) {
    throw new Error(`Query ${queryName} failed: ${res.status}`)
  }
  return (await res.json()) as T
}
