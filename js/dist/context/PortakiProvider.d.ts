import type { ReactNode } from 'react';
import type { PortakiContext } from '../types';
export type PortakiProviderProps = {
    context: PortakiContext;
    /** Server-rendered per-session HMAC key (opaque to callers). */
    hmacKeyMaterialB64: string;
    children: ReactNode;
};
export declare function PortakiProvider({ context, hmacKeyMaterialB64, children }: PortakiProviderProps): import("react/jsx-runtime").JSX.Element;
//# sourceMappingURL=PortakiProvider.d.ts.map