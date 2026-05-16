import type { PortakiContext } from '@portaki/sdk';
import type { HostModuleContext, LangCode, ModuleContext, PropertyData, StayData } from '@portaki/module-sdk';
export type MockPortakiRuntimeValue = PortakiContext & {
    hmacKeyMaterialB64: string;
};
export declare const FIXTURE_STAY: StayData;
export declare const FIXTURE_PROPERTY: PropertyData;
export type MockModuleContextOverrides = {
    stay?: Partial<StayData>;
    property?: Partial<PropertyData>;
    lang?: LangCode;
    config?: Record<string, string | boolean | number>;
};
export declare function createSpyTrack(): ModuleContext['track'];
export declare function createMockModuleContext(overrides?: MockModuleContextOverrides): ModuleContext;
export declare function createMockPortakiRuntimeValue(overrides?: Partial<MockPortakiRuntimeValue>): MockPortakiRuntimeValue;
export declare function createMockHostModuleContext(overrides?: Partial<HostModuleContext> & {
    propertyId?: string;
}): HostModuleContext;
//# sourceMappingURL=fixtures.d.ts.map