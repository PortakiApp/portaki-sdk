export type HmacPayload = {
    moduleId: string;
    queryName?: string;
    commandName?: string;
    stayId: string;
    timestamp: number;
};
export declare function decodeModuleHmacKeyMaterial(b64: string): Uint8Array;
export declare function signHmacPayload(keyMaterial: Uint8Array, payload: HmacPayload): Promise<string>;
//# sourceMappingURL=hmac.d.ts.map