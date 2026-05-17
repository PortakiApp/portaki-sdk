import { jsx as _jsx } from "react/jsx-runtime";
import { render } from '@testing-library/react';
import { PortakiProvider } from '@portaki/sdk';
import { createMockHostModuleContext, createMockModuleContext, createMockPortakiRuntimeValue, } from './fixtures.js';
export function renderGuestModule(module, options = {}) {
    const ctx = createMockModuleContext(options);
    const ui = module.render(ctx);
    const runtime = createMockPortakiRuntimeValue({ moduleId: module.id, lang: ctx.lang });
    const { hmacKeyMaterialB64, ...portakiContext } = runtime;
    const Wrapper = ({ children }) => (_jsx(PortakiProvider, { context: portakiContext, hmacKeyMaterialB64: hmacKeyMaterialB64, children: children }));
    const result = render(ui, { wrapper: Wrapper, ...options.renderOptions });
    return { ...result, ctx };
}
export function renderHostModule(module, ctx = createMockHostModuleContext(), renderOptions) {
    if (!module.renderHost) {
        throw new Error(`module ${module.id} has no renderHost`);
    }
    const ui = module.renderHost(ctx);
    const result = render(ui, renderOptions);
    return { ...result, ctx };
}
