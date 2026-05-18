import '@testing-library/jest-dom/vitest';
import * as React from 'react';
import { vi } from 'vitest';
globalThis.React = React;
vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response(JSON.stringify({ sections: [], items: [] }), {
    status: 200,
    headers: { 'content-type': 'application/json' },
})));
