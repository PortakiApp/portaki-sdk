import { type ReactNode } from 'react';
import type { PortakiContext } from '../types';
export declare const PortakiContextInternal: import("react").Context<PortakiContext | null>;
export type PortakiProviderValue = PortakiContext & {
    /** Base64url-encoded raw HMAC key material (server-issued, stay-scoped). */
    hmacKeyMaterialB64: string;
};
export declare function PortakiRuntimeProvider({ value, children, }: {
    value: PortakiProviderValue;
    children: ReactNode;
}): import("react/jsx-runtime").JSX.Element;
export declare function usePortakiRuntime(): PortakiProviderValue;
//# sourceMappingURL=portaki-internal-context.d.ts.map