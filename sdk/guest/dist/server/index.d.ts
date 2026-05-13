import { deriveModuleHmacKeyMaterialB64 } from './derive-key';
import { verifyHmacToken } from './verify-hmac';
import { signHmacTokenNode } from './sign-hmac-node';
export { verifyHmacToken, deriveModuleHmacKeyMaterialB64, signHmacTokenNode };
export declare function portakiServerQuery<T>(queryName: string, params: Record<string, unknown>, context: {
    moduleId: string;
    stayId: string;
}, opts: {
    hmacKeyMaterialB64: string;
    baseUrl: string;
}): Promise<T>;
//# sourceMappingURL=index.d.ts.map