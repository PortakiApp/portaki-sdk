export const FIXTURE_STAY = {
    id: 'stay-test-001',
    guestName: 'Marie Dupont',
    guestLang: 'fr',
    checkinAt: '2026-06-01T15:00:00.000Z',
    checkoutAt: '2026-06-08T10:00:00.000Z',
    accessCode: 'ABCD12',
    status: 'ACTIVE',
};
export const FIXTURE_PROPERTY = {
    id: 'property-test-001',
    name: 'Villa Test',
    slug: 'villa-test',
    address: '1 rue de la Plage, 13008 Marseille',
    lat: 43.25,
    lng: 5.38,
    checklistItems: [
        { id: 'trash', labelFr: 'Sortir les poubelles', labelEn: 'Take out trash' },
        { id: 'windows', labelFr: 'Fermer les fenêtres', labelEn: 'Close windows' },
    ],
};
export function createSpyTrack() {
    const events = [];
    const track = (event) => {
        events.push({ type: event.type, label: event.label });
    };
    return Object.assign(track, { events });
}
export function createMockModuleContext(overrides = {}) {
    return {
        stay: { ...FIXTURE_STAY, ...overrides.stay },
        property: { ...FIXTURE_PROPERTY, ...overrides.property },
        lang: overrides.lang ?? 'fr',
        config: overrides.config ?? {},
        track: createSpyTrack(),
    };
}
export function createMockPortakiRuntimeValue(overrides = {}) {
    return {
        stay: {
            id: FIXTURE_STAY.id,
            guestName: FIXTURE_STAY.guestName,
            checkinAt: FIXTURE_STAY.checkinAt,
            checkoutAt: FIXTURE_STAY.checkoutAt,
            checkinTime: '15:00',
            checkoutTime: '10:00',
            status: 'ACTIVE',
            lang: 'fr',
        },
        property: {
            id: FIXTURE_PROPERTY.id,
            name: FIXTURE_PROPERTY.name ?? 'Villa Test',
            address: FIXTURE_PROPERTY.address ?? '',
            lat: FIXTURE_PROPERTY.lat ?? 0,
            lng: FIXTURE_PROPERTY.lng ?? 0,
            theme: { primaryHex: '#E8724A' },
        },
        lang: 'fr',
        config: {},
        scopes: ['stay:read'],
        moduleId: 'test-module',
        isPreview: true,
        hmacKeyMaterialB64: 'dGVzdC1rZXk=',
        ...overrides,
    };
}
export function createMockHostModuleContext(overrides = {}) {
    return {
        lang: overrides.lang ?? 'fr',
        propertyId: overrides.propertyId ?? FIXTURE_PROPERTY.id,
        config: overrides.config ?? {},
        track: overrides.track ?? createSpyTrack(),
    };
}
