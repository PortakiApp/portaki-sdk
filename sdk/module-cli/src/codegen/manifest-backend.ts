import type { PortakiFullModule } from '@portaki/module-sdk/module'

export type ManifestGatewayQuery = {
  name: string
  scope: string
  description?: { fr: string; en: string }
}

export type ManifestGatewayCommand = {
  name: string
  scope: string
  description?: { fr: string; en: string }
}

export function gatewayManifestSlice(module: PortakiFullModule): {
  database?: { schemaVersion: string }
  queries?: ManifestGatewayQuery[]
  commands?: ManifestGatewayCommand[]
  scopes: string[]
} {
  if (!module.backend) {
    return { scopes: [] }
  }

  const scopeSet = new Set<string>()
  const queries: ManifestGatewayQuery[] = []
  const commands: ManifestGatewayCommand[] = []

  for (const [name, def] of Object.entries(module.backend.queries)) {
    scopeSet.add(def.scope)
    queries.push({
      name,
      scope: def.scope,
      description: def.description,
    })
  }

  for (const [name, def] of Object.entries(module.backend.commands)) {
    scopeSet.add(def.scope)
    commands.push({
      name,
      scope: def.scope,
      description: def.description,
    })
  }

  return {
    database: { schemaVersion: module.backend.schemaVersion },
    queries: queries.length > 0 ? queries : undefined,
    commands: commands.length > 0 ? commands : undefined,
    scopes: [...scopeSet].sort(),
  }
}
