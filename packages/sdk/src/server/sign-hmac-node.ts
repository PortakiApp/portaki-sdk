import { createHmac } from 'node:crypto'

import type { HmacPayload } from '../runtime/security/hmac.js'
import { decodeModuleHmacKeyMaterial } from '../runtime/security/hmac.js'

function toBase64Url(buf: Buffer): string {
  return buf
    .toString('base64')
    .replace(/\+/g, '-')
    .replace(/\//g, '_')
    .replace(/=+$/, '')
}

/** Same wire format as `signHmacPayload` in the browser — for tests and server-side callers. */
export function signHmacTokenNode(keyMaterialB64: string, payload: HmacPayload): string {
  const raw = decodeModuleHmacKeyMaterial(keyMaterialB64)
  const key = Buffer.from(raw)
  const msgBytes = Buffer.from(JSON.stringify(payload), 'utf8')
  const mac = createHmac('sha256', key)
  mac.update(msgBytes)
  return `${toBase64Url(mac.digest())}.${toBase64Url(msgBytes)}`
}
