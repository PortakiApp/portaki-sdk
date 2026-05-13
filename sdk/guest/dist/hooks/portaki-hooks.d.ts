import type { PortakiContext, PortakiError, QueryResult } from '../types';
export declare function usePortakiContext(): PortakiContext;
export declare function usePortakiConfig<T extends Record<string, unknown>>(): T;
export declare function usePortakiQuery<T>(queryName: string, params?: Record<string, unknown>, options?: {
    enabled?: boolean;
    refetchInterval?: number;
}): QueryResult<T>;
export declare function usePortakiCommand<TParams = Record<string, unknown>>(commandName: string): {
    execute: (params: TParams) => Promise<void>;
    loading: boolean;
    error: PortakiError | null;
};
export declare function usePortakiModuleQuery<T>(options: {
    moduleId: string;
    queryName: string;
    params?: Record<string, unknown>;
    enabled?: boolean;
}): QueryResult<T>;
//# sourceMappingURL=portaki-hooks.d.ts.map