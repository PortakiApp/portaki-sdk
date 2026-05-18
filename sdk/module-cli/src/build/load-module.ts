import { createJiti } from 'jiti'

import type { PortakiFullModule } from '@portaki/module-sdk/module'

export async function loadModuleDefinition(entryPath: string): Promise<PortakiFullModule> {
  const jiti = createJiti(import.meta.url, {
    interopDefault: true,
    moduleCache: false,
  })
  const loaded = (await jiti.import(entryPath)) as Record<string, unknown>
  const mod = (loaded.default ?? loaded.module) as PortakiFullModule | undefined
  if (mod == null || typeof mod !== 'object' || typeof mod.id !== 'string') {
    throw new Error(
      `Module entry must export default defineModule(...): ${entryPath}`,
    )
  }
  return mod as PortakiFullModule
}
