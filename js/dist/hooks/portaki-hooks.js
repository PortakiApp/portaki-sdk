'use client';
import { useCallback, useContext, useEffect, useMemo, useState } from 'react';
import { PortakiContextInternal, usePortakiRuntime } from '../context/portaki-internal-context';
import { decodeModuleHmacKeyMaterial, signHmacPayload } from '../security/hmac';
export function usePortakiContext() {
    const ctx = useContext(PortakiContextInternal);
    if (!ctx) {
        throw new Error('usePortakiContext must be used inside a PortakiProvider');
    }
    return ctx;
}
export function usePortakiConfig() {
    const { config } = usePortakiContext();
    return config;
}
export function usePortakiQuery(queryName, params, options) {
    const { moduleId, stay, hmacKeyMaterialB64 } = usePortakiRuntime();
    const [data, setData] = useState(null);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState(null);
    const keyMaterial = useMemo(() => decodeModuleHmacKeyMaterial(hmacKeyMaterialB64), [hmacKeyMaterialB64]);
    const paramsKey = useMemo(() => JSON.stringify(params ?? {}), [params]);
    const runQuery = useCallback(async () => {
        if (options?.enabled === false) {
            setLoading(false);
            return;
        }
        setLoading(true);
        try {
            const token = await signHmacPayload(keyMaterial, {
                moduleId,
                queryName,
                stayId: stay.id,
                timestamp: Date.now(),
            });
            const res = await globalThis.fetch('/api/portaki/query', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                    'X-Portaki-Module': moduleId,
                    'X-Portaki-Token': token,
                },
                body: JSON.stringify({
                    query: queryName,
                    params: { ...params, _stayId: stay.id },
                }),
            });
            if (!res.ok) {
                let errBody = {};
                try {
                    errBody = await res.json();
                }
                catch {
                    /* ignore */
                }
                setError({
                    code: errBody.code ?? 'REQUEST_FAILED',
                    message: errBody.message ?? res.statusText,
                });
                setData(null);
                return;
            }
            setData((await res.json()));
            setError(null);
        }
        catch {
            setError({ code: 'NETWORK_ERROR', message: 'Network error' });
            setData(null);
        }
        finally {
            setLoading(false);
        }
    }, [keyMaterial, moduleId, options?.enabled, params, paramsKey, queryName, stay.id]);
    useEffect(() => {
        void runQuery();
        if (!options?.refetchInterval) {
            return undefined;
        }
        const id = setInterval(() => void runQuery(), options.refetchInterval);
        return () => clearInterval(id);
    }, [runQuery, options?.refetchInterval]);
    return { data, loading, error, refetch: runQuery };
}
export function usePortakiCommand(commandName) {
    const { moduleId, stay, hmacKeyMaterialB64 } = usePortakiRuntime();
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState(null);
    const keyMaterial = useMemo(() => decodeModuleHmacKeyMaterial(hmacKeyMaterialB64), [hmacKeyMaterialB64]);
    const execute = useCallback(async (params) => {
        setLoading(true);
        try {
            const token = await signHmacPayload(keyMaterial, {
                moduleId,
                commandName,
                stayId: stay.id,
                timestamp: Date.now(),
            });
            const res = await globalThis.fetch('/api/portaki/command', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                    'X-Portaki-Module': moduleId,
                    'X-Portaki-Token': token,
                },
                body: JSON.stringify({
                    command: commandName,
                    params: { ...params, _stayId: stay.id },
                }),
            });
            if (!res.ok) {
                let msg = res.statusText;
                try {
                    const errBody = await res.json();
                    msg = errBody.message ?? msg;
                }
                catch {
                    /* ignore */
                }
                throw new Error(msg);
            }
            setError(null);
        }
        catch (e) {
            const message = e instanceof Error ? e.message : 'Command failed';
            setError({ code: 'COMMAND_ERROR', message });
            throw e;
        }
        finally {
            setLoading(false);
        }
    }, [commandName, keyMaterial, moduleId, stay.id]);
    return { execute, loading, error };
}
export function usePortakiModuleQuery(options) {
    const { stay, hmacKeyMaterialB64 } = usePortakiRuntime();
    const [data, setData] = useState(null);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState(null);
    const keyMaterial = useMemo(() => decodeModuleHmacKeyMaterial(hmacKeyMaterialB64), [hmacKeyMaterialB64]);
    const fullQuery = `${options.moduleId}.${options.queryName}`;
    const paramsKey = useMemo(() => JSON.stringify(options.params ?? {}), [options.params]);
    const runDepQuery = useCallback(async () => {
        if (options.enabled === false) {
            setLoading(false);
            return;
        }
        setLoading(true);
        try {
            const token = await signHmacPayload(keyMaterial, {
                moduleId: options.moduleId,
                queryName: fullQuery,
                stayId: stay.id,
                timestamp: Date.now(),
            });
            const res = await globalThis.fetch('/api/portaki/query', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                    'X-Portaki-Module': options.moduleId,
                    'X-Portaki-Token': token,
                },
                body: JSON.stringify({
                    query: fullQuery,
                    params: { ...options.params, _stayId: stay.id },
                }),
            });
            if (!res.ok) {
                setData(null);
                let errBody = {};
                try {
                    errBody = await res.json();
                }
                catch {
                    /* ignore */
                }
                setError({
                    code: errBody.code ?? 'REQUEST_FAILED',
                    message: errBody.message ?? res.statusText,
                });
                return;
            }
            setData((await res.json()));
            setError(null);
        }
        catch {
            setError({ code: 'NETWORK_ERROR', message: 'Network error' });
            setData(null);
        }
        finally {
            setLoading(false);
        }
    }, [fullQuery, keyMaterial, options.enabled, options.moduleId, options.params, paramsKey, stay.id]);
    useEffect(() => {
        void runDepQuery();
    }, [runDepQuery]);
    return { data, loading, error, refetch: runDepQuery };
}
