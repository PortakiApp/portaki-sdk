import type { ComponentType } from 'react';
import type { SlotDefinition } from '../types';
export type SlotRegistration = {
    moduleId: string;
    slot: SlotDefinition;
    component: ComponentType<Record<string, unknown>>;
};
declare class SlotRegistry {
    private slots;
    register(moduleId: string, slot: SlotDefinition, component: ComponentType<Record<string, unknown>>): void;
    getSlot(name: string): SlotRegistration[];
}
export declare const slotRegistry: SlotRegistry;
export declare const portaki: {
    slot: (moduleId: string, slot: SlotDefinition, component: ComponentType<Record<string, unknown>>) => void;
};
export {};
//# sourceMappingURL=slot-registry.d.ts.map