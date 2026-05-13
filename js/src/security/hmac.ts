function base64ToUint8Array(b64: string): Uint8Array {
  const bin = atob(b64.replace(/-/g, '+').replace(/_/g, '/'))
  const out = new Uint8Array(bin.length)
  for (let i = 0; i < bin.length; i++) {
    out[i] = bin.charCodeAt(i)
  }
  return out
}

function uint8ArrayToBase64Url(u8: Uint8Array): string {
  let bin = ''
  for (let i = 0; i < u8.length; i++) {
    bin += String.fromCharCode(u8[i])
  }
  const b64 = btoa(bin)
  return b64.replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '')
}

async function importHmacKey(raw: Uint8Array): Promise<CryptoKey> {
  return crypto.subtle.importKey('raw', raw as BufferSource, { name: 'HMAC', hash: 'SHA-256' }, false, [
    'sign',
  ])
}

export type HmacPayload = {
  moduleId: string
  queryName?: string
  commandName?: string
  stayId: string
  timestamp: number
}

export function decodeModuleHmacKeyMaterial(b64: string): Uint8Array {
  return base64ToUint8Array(b64)
}

export async function signHmacPayload(keyMaterial: Uint8Array, payload: HmacPayload): Promise<string> {
  const key = await importHmacKey(keyMaterial)
  const message = JSON.stringify(payload)
  const msgBytes = new TextEncoder().encode(message)
  const sig = await crypto.subtle.sign('HMAC', key, msgBytes)
  const sigPart = uint8ArrayToBase64Url(new Uint8Array(sig))
  const msgPart = uint8ArrayToBase64Url(msgBytes)
  return `${sigPart}.${msgPart}`
}
