import { decodeModuleHmacKeyMaterial, signHmacPayload } from '../security/hmac';
import { deriveModuleHmacKeyMaterialB64 } from './derive-key';
import { verifyHmacToken } from './verify-hmac';
import { signHmacTokenNode } from './sign-hmac-node';
export { verifyHmacToken, deriveModuleHmacKeyMaterialB64, signHmacTokenNode };
export async function portakiServerQuery(queryName, params, context, opts) {
    const keyMaterial = decodeModuleHmacKeyMaterial(opts.hmacKeyMaterialB64);
    const token = await signHmacPayload(keyMaterial, {
        moduleId: context.moduleId,
        queryName,
        stayId: context.stayId,
        timestamp: Date.now(),
    });
    const base = opts.baseUrl.replace(/\/$/, '');
    const res = await fetch(`${base}/api/portaki/query`, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
            'X-Portaki-Module': context.moduleId,
            'X-Portaki-Token': token,
        },
        body: JSON.stringify({ query: queryName, params: { ...params, _stayId: context.stayId } }),
        cache: 'no-store',
    });
    if (!res.ok) {
        throw new Error(`Query ${queryName} failed: ${res.status}`);
    }
    return (await res.json());
}
