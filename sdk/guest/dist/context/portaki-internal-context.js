'use client';
import { jsx as _jsx } from "react/jsx-runtime";
import { createContext, useContext } from 'react';
export const PortakiContextInternal = createContext(null);
const PortakiRuntimeContext = createContext(null);
export function PortakiRuntimeProvider({ value, children, }) {
    const { hmacKeyMaterialB64: _k, ...ctx } = value;
    return (_jsx(PortakiRuntimeContext.Provider, { value: value, children: _jsx(PortakiContextInternal.Provider, { value: ctx, children: children }) }));
}
export function usePortakiRuntime() {
    const v = useContext(PortakiRuntimeContext);
    if (!v) {
        throw new Error('PortakiRuntimeProvider is missing');
    }
    return v;
}
