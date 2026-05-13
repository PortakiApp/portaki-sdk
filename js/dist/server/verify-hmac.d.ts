export declare function verifyHmacToken(token: string | null, expected: {
    moduleId: string;
    queryName?: string;
    commandName?: string;
    stayId: string;
}, keyMaterialB64: string, maxSkewMs?: number): boolean;
//# sourceMappingURL=verify-hmac.d.ts.map