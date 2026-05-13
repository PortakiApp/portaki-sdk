'use client';
class SlotRegistry {
    slots = new Map();
    register(moduleId, slot, component) {
        const key = slot.name;
        const existing = this.slots.get(key) ?? [];
        this.slots.set(key, [...existing, { moduleId, slot, component }]);
    }
    getSlot(name) {
        return this.slots.get(name) ?? [];
    }
}
export const slotRegistry = new SlotRegistry();
export const portaki = {
    slot: (moduleId, slot, component) => {
        slotRegistry.register(moduleId, slot, component);
    },
};
