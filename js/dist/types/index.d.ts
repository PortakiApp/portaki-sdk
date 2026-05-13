export type PortakiLang = 'fr' | 'en';
export interface PropertyTheme {
    primaryHex: string;
    accentHex?: string;
}
export interface StayData {
    id: string;
    guestName: string;
    checkinAt: string;
    checkoutAt: string;
    checkinTime: string;
    checkoutTime: string;
    status: 'PRE_ARRIVAL' | 'UPCOMING' | 'ACTIVE' | 'COMPLETED';
    lang: PortakiLang;
}
export interface PropertyData {
    id: string;
    name: string;
    address: string;
    lat: number;
    lng: number;
    theme: PropertyTheme;
}
export interface PortakiContext {
    stay: StayData;
    property: PropertyData;
    lang: PortakiLang;
    config: Record<string, string | boolean | number>;
    scopes: string[];
    moduleId: string;
    isPreview: boolean;
}
export interface PortakiError {
    code: string;
    message: string;
}
export interface QueryResult<T> {
    data: T | null;
    loading: boolean;
    error: PortakiError | null;
    refetch: () => void;
}
export type SlotName = 'section' | 'bottom-bar' | 'map-overlay' | 'post-stay';
export interface SlotDefinition {
    name: SlotName;
    position?: string;
    defaultLabel?: {
        fr: string;
        en: string;
    };
    defaultIcon?: string;
}
export interface DependencyQueryOptions {
    moduleId: string;
    queryName: string;
    params?: Record<string, unknown>;
}
//# sourceMappingURL=index.d.ts.map