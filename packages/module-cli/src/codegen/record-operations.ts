import type { PortakiFullModule } from '@portaki/sdk/author'
import { createGatewayContext } from '@portaki/sdk/backend'
import { createRecordingModuleDb } from '@portaki/sdk/backend/recording-db'

export type OperationStep = {
  readonly kind: 'queryOne' | 'query' | 'execute'
  readonly sql: string
  readonly params: readonly unknown[]
  readonly paramKeys?: readonly string[]
}

export type OperationDefinition = {
  readonly scope: string
  readonly steps: readonly OperationStep[]
}

export type OperationsBundle = {
  readonly moduleId: string
  readonly moduleVersion: string
  readonly schemaVersion: string
  readonly operations: Record<string, OperationDefinition>
}

const MOCK_TENANT = '00000000-0000-4000-8000-000000000001'
const MOCK_PROPERTY = '00000000-0000-4000-8000-000000000002'
const MOCK_STAY = '00000000-0000-4000-8000-000000000003'

export async function recordOperationsBundle(module: PortakiFullModule): Promise<OperationsBundle> {
  if (!module.backend) {
    return {
      moduleId: module.id,
      moduleVersion: module.version,
      schemaVersion: '0',
      operations: {},
    }
  }

  const { db, database } = createRecordingModuleDb(module.backend.schema)
  const operations: Record<string, OperationDefinition> = {}

  for (const [name, def] of Object.entries(module.backend.queries)) {
    database.calls.length = 0
    const ctx = createGatewayContext({
      moduleId: module.id,
      moduleVersion: module.version,
      tenantId: MOCK_TENANT,
      propertyId: MOCK_PROPERTY,
      stayId: MOCK_STAY,
      scopes: [def.scope],
      config: {},
      schema: module.backend.schema,
      database,
    })
    await def.handler(ctx, {})
    operations[name] = {
      scope: def.scope,
      steps: toSteps(database.calls, ['tenantId', 'propertyId']),
    }
  }

  for (const [name, def] of Object.entries(module.backend.commands)) {
    const keys = commandParamKeys(name)
    operations[name] = {
      scope: def.scope,
      steps: await recordCommandSteps(def, module, database, keys, name),
    }
  }

  return {
    moduleId: module.id,
    moduleVersion: module.version,
    schemaVersion: module.backend.schemaVersion,
    operations,
  }
}

function toSteps(
  calls: readonly { kind: 'query' | 'queryOne' | 'execute'; sql: string; params: readonly unknown[] }[],
  paramKeys: readonly string[],
): OperationStep[] {
  return calls.map((call) => ({
    kind: call.kind,
    sql: call.sql,
    params: call.params,
    paramKeys: paramKeys.length > 0 ? paramKeys : undefined,
  }))
}

function sampleCommandParams(commandName: string): Record<string, unknown> {
  if (commandName.includes('save')) {
    return {
      contentFr: { type: 'doc', content: [] },
      contentEn: { type: 'doc', content: [] },
    }
  }
  return {}
}

async function recordCommandSteps(
  def: { readonly scope: string; readonly handler: (ctx: unknown, params: Record<string, unknown>) => Promise<void> },
  module: PortakiFullModule,
  database: ReturnType<typeof createRecordingModuleDb>['database'],
  paramKeys: readonly string[],
  commandName: string,
): Promise<OperationStep[]> {
  const allSteps: OperationStep[] = []

  const run = async () => {
    database.calls.length = 0
    const ctx = createGatewayContext({
      moduleId: module.id,
      moduleVersion: module.version,
      tenantId: MOCK_TENANT,
      propertyId: MOCK_PROPERTY,
      stayId: null,
      scopes: [def.scope],
      config: {},
      schema: module.backend!.schema,
      database,
    })
    await def.handler(ctx, sampleCommandParams(commandName))
    allSteps.push(...toSteps(database.calls, paramKeys))
  }

  await run()
  if (commandName.includes('save')) {
    database.seedQueryOneResponses({ id: '00000000-0000-4000-8000-000000000099' })
    await run()
  }

  return dedupeSteps(allSteps)
}

function dedupeSteps(steps: OperationStep[]): OperationStep[] {
  const seen = new Set<string>()
  const out: OperationStep[] = []
  for (const step of steps) {
    const key = `${step.kind}:${step.sql}`
    if (seen.has(key)) {
      continue
    }
    seen.add(key)
    out.push(step)
  }
  return out
}

function commandParamKeys(commandName: string): readonly string[] {
  if (commandName.includes('save')) {
    return ['tenantId', 'propertyId', 'contentFr', 'contentEn']
  }
  return ['tenantId', 'propertyId']
}
