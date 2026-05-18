import { type RenderOptions, type RenderResult } from '@testing-library/react';
import type { HostModuleContext, ModuleContext, PortakiModuleDefinition } from '@portaki/sdk';
import { type MockModuleContextOverrides } from './fixtures.js';
export type RenderGuestModuleOptions = MockModuleContextOverrides & {
    renderOptions?: Omit<RenderOptions, 'wrapper'>;
};
export declare function renderGuestModule(module: PortakiModuleDefinition, options?: RenderGuestModuleOptions): RenderResult & {
    ctx: ModuleContext;
};
export declare function renderHostModule(module: PortakiModuleDefinition, ctx?: HostModuleContext, renderOptions?: Omit<RenderOptions, 'wrapper'>): RenderResult & {
    ctx: HostModuleContext;
};
//# sourceMappingURL=render-module.d.ts.map