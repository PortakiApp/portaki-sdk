'use client';
import { jsx as _jsx } from "react/jsx-runtime";
import { PortakiRuntimeProvider } from './portaki-internal-context';
export function PortakiProvider({ context, hmacKeyMaterialB64, children }) {
    const value = { ...context, hmacKeyMaterialB64 };
    return _jsx(PortakiRuntimeProvider, { value: value, children: children });
}
