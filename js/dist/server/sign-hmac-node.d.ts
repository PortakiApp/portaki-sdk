import type { HmacPayload } from '../security/hmac';
/** Same wire format as `signHmacPayload` in the browser — for tests and server-side callers. */
export declare function signHmacTokenNode(keyMaterialB64: string, payload: HmacPayload): string;
//# sourceMappingURL=sign-hmac-node.d.ts.map