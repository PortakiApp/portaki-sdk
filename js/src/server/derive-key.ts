import { createHmac } from 'crypto'

function toBase64Url(buf: Buffer): string {
  return buf
    .toString('base64')
    .replace(/\+/g, '-')
    .replace(/\//g, '_')
    .replace(/=+$/, '')
}

export function deriveModuleHmacKeyMaterialB64(
  masterSecret: string,
  moduleId: string,
  stayId: string,
): string {
  const mac = createHmac('sha256', Buffer.from(masterSecret, 'utf8'))
  mac.update(`${moduleId}|${stayId}`, 'utf8')
  return toBase64Url(mac.digest())
}
