import { createHmac, timingSafeEqual } from 'crypto';
import { decodeModuleHmacKeyMaterial } from '../security/hmac';
function base64UrlToBuffer(s) {
    const pad = s.length % 4 === 0 ? '' : '='.repeat(4 - (s.length % 4));
    const b64 = s.replace(/-/g, '+').replace(/_/g, '/') + pad;
    return Buffer.from(b64, 'base64');
}
export function verifyHmacToken(token, expected, keyMaterialB64, maxSkewMs = 120_000) {
    if (!token || !token.includes('.')) {
        return false;
    }
    const [sigPart, msgPart] = token.split('.');
    if (!sigPart || !msgPart) {
        return false;
    }
    let payload;
    try {
        const json = base64UrlToBuffer(msgPart).toString('utf8');
        payload = JSON.parse(json);
    }
    catch {
        return false;
    }
    if (payload.moduleId !== expected.moduleId) {
        return false;
    }
    if (expected.queryName != null && payload.queryName !== expected.queryName) {
        return false;
    }
    if (expected.commandName != null && payload.commandName !== expected.commandName) {
        return false;
    }
    if (payload.stayId !== expected.stayId) {
        return false;
    }
    if (Math.abs(Date.now() - payload.timestamp) > maxSkewMs) {
        return false;
    }
    const rawKey = decodeModuleHmacKeyMaterial(keyMaterialB64);
    const key = Buffer.from(rawKey);
    const msgBytes = Buffer.from(JSON.stringify(payload), 'utf8');
    const mac = createHmac('sha256', key);
    mac.update(msgBytes);
    const expectedSig = mac.digest();
    let actual;
    try {
        actual = base64UrlToBuffer(sigPart);
    }
    catch {
        return false;
    }
    if (actual.length !== expectedSig.length) {
        return false;
    }
    return timingSafeEqual(actual, expectedSig);
}
