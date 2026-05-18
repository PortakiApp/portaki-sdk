import { mkdir, writeFile } from 'node:fs/promises'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

import type { PortakiFullModule } from '@portaki/sdk'
import * as esbuild from 'esbuild'

export type ExtensionBundle = {
  readonly moduleId: string
  readonly moduleVersion: string
  readonly hostActions: readonly string[]
  readonly eventHandlers: readonly string[]
}

export async function bundleModuleExtension(
  module: PortakiFullModule,
  entryPath: string,
  outDir: string,
): Promise<ExtensionBundle | null> {
  const hostActions = Object.keys(module.hostActions ?? {})
  const eventHandlers = Object.keys(module.eventHandlers ?? {})
  if (hostActions.length === 0 && eventHandlers.length === 0) {
    return null
  }

  await mkdir(outDir, { recursive: true })
  const shimPath = join(outDir, '.extension-shim.mjs')
  const entryAbs = entryPath.startsWith('/') ? entryPath : join(process.cwd(), entryPath)
  const shim = `
import * as mod from ${JSON.stringify(entryAbs)};

const definition = mod.default ?? mod;
const hostActions = definition.hostActions ?? mod.hostActions ?? {};
const eventHandlers = definition.eventHandlers ?? mod.eventHandlers ?? {};

export async function runHostAction(action, ctx, plainConfig) {
  const handler = hostActions[action];
  if (typeof handler !== 'function') {
    throw new Error('host_action_not_found: ' + action);
  }
  return await handler(ctx, plainConfig);
}

export async function handleEvent(eventName, ctx, eventData) {
  const handler = eventHandlers[eventName];
  if (typeof handler !== 'function') {
    return;
  }
  await handler(ctx, eventData);
}
`
  await writeFile(shimPath, shim, 'utf8')

  const extensionCjs = join(outDir, 'extension.cjs')
  await esbuild.build({
    entryPoints: [shimPath],
    bundle: true,
    platform: 'node',
    format: 'cjs',
    target: 'node20',
    outfile: extensionCjs,
    logLevel: 'silent',
    external: ['react', 'react-dom', 'react/jsx-runtime', 'react/jsx-dev-runtime'],
    absWorkingDir: dirname(fileURLToPath(import.meta.url)),
  })

  const bundle: ExtensionBundle = {
    moduleId: module.id,
    moduleVersion: module.version,
    hostActions,
    eventHandlers,
  }
  await writeFile(join(outDir, 'extension.bundle.json'), JSON.stringify(bundle, null, 2) + '\n')
  return bundle
}
