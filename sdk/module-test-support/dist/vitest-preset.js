import { existsSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vitest/config';
const setupFile = join(dirname(fileURLToPath(import.meta.url)), 'vitest-setup.js');
function resolvePackageSrcEntry(moduleRoot, packageName, entry = 'src/index.ts') {
    const linkedSrc = join(moduleRoot, 'node_modules', packageName, entry);
    if (existsSync(linkedSrc)) {
        return linkedSrc;
    }
    try {
        const require = createRequire(join(moduleRoot, 'package.json'));
        const resolved = require.resolve(packageName);
        const srcSibling = join(dirname(resolved), '..', entry);
        if (existsSync(srcSibling)) {
            return srcSibling;
        }
        return resolved;
    }
    catch {
        const fallbackByPackage = {
            '@portaki/module-sdk': '../../module-sdk/src/index.ts',
            '@portaki/sdk': '../../guest/src/index.ts',
        };
        const rel = fallbackByPackage[packageName] ?? entry;
        return join(dirname(fileURLToPath(import.meta.url)), rel);
    }
}
/**
 * Preset Vitest pour les packages `@portaki/module-*`.
 * Usage : `export default portakiModuleVitestConfig(import.meta.url)`
 */
export function portakiModuleVitestConfig(moduleEntryUrl) {
    const moduleRoot = dirname(fileURLToPath(moduleEntryUrl));
    const moduleSdkEntry = resolvePackageSrcEntry(moduleRoot, '@portaki/module-sdk');
    const guestSdkEntry = resolvePackageSrcEntry(moduleRoot, '@portaki/sdk');
    return defineConfig({
        root: moduleRoot,
        resolve: {
            alias: {
                '@portaki/module-sdk': moduleSdkEntry,
                '@portaki/sdk': guestSdkEntry,
            },
        },
        test: {
            environment: 'jsdom',
            setupFiles: [setupFile],
            include: ['src/**/*.test.ts', 'src/**/*.test.tsx'],
            passWithNoTests: false,
            server: {
                deps: {
                    inline: ['@portaki/module-sdk', '@portaki/sdk', '@portaki/module-test-support'],
                },
            },
        },
    });
}
export default portakiModuleVitestConfig;
