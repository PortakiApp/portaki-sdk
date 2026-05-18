import type { CommandDefinition, PortakiFullModule, QueryDefinition } from '@portaki/sdk'
import type { ModuleSchemaDef } from '@portaki/sdk'
import { createHandlerContext, createRecordingModuleDb } from '@portaki/sdk/build'

const EMPTY_SCHEMA: ModuleSchemaDef = { tables: [] }

export type OperationStep = {
  readonly kind: 'queryOne' | 'query' | 'execute'
  readonly sql: string
  readonly params: readonly unknown[]
  readonly paramKeys?: readonly string[]
}

export type OperationDefinition = {
  readonly scope: string
  readonly steps: readonly OperationStep[]
  readonly events?: readonly { readonly name: string; readonly payload: Record<string, unknown> }[]
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
  if (!module.data) {
    return {
      moduleId: module.id,
      moduleVersion: module.version,
      schemaVersion: '0',
      operations: {},
    }
  }

  const schema = module.data.schema ?? EMPTY_SCHEMA
  const { database } = createRecordingModuleDb(schema)
  const operations: Record<string, OperationDefinition> = {}

  for (const [name, def] of Object.entries(module.data.queries) as [string, QueryDefinition][]) {
    database.calls.length = 0
    const ctx = createHandlerContext({
      moduleId: module.id,
      moduleVersion: module.version,
      tenantId: MOCK_TENANT,
      propertyId: MOCK_PROPERTY,
      stayId: MOCK_STAY,
      scopes: [def.scope],
      config: {},
      schema,
      database,
    })
    await def.handler(ctx, {})
    operations[name] = {
      scope: def.scope,
      steps: toSteps(database.calls, ['tenantId', 'propertyId']),
    }
  }

  for (const [name, def] of Object.entries(module.data.commands) as [string, CommandDefinition][]) {
    const keys = commandParamKeys(name)
    const recorded = await recordCommandSteps(def, module, database, keys, name)
    operations[name] = {
      scope: def.scope,
      steps: recorded.steps,
      ...(recorded.events.length > 0 ? { events: recorded.events } : {}),
    }
  }

  return {
    moduleId: module.id,
    moduleVersion: module.version,
    schemaVersion: module.data.schemaVersion,
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
): Promise<{
  steps: OperationStep[]
  events: { name: string; payload: Record<string, unknown> }[]
}> {
  const allSteps: OperationStep[] = []
  const publishedEvents: { name: string; payload: Record<string, unknown> }[] = []

  const run = async () => {
    database.calls.length = 0
    publishedEvents.length = 0
    const ctx = createHandlerContext({
      moduleId: module.id,
      moduleVersion: module.version,
      tenantId: MOCK_TENANT,
      propertyId: MOCK_PROPERTY,
      stayId: commandUsesStay(commandName, def.scope) ? MOCK_STAY : null,
      scopes: [def.scope],
      config: {},
      schema: module.data!.schema,
      database,
      onPublish: (event) => {
        publishedEvents.push(event)
      },
    })
    await def.handler(ctx, sampleCommandParams(commandName))
    allSteps.push(...toSteps(database.calls, paramKeys))
  }

  await run()
  if (commandName.includes('save') || commandName.includes('submit')) {
    database.seedQueryOneResponses({ id: '00000000-0000-4000-8000-000000000099' })
    await run()
  }

  return { steps: dedupeSteps(allSteps), events: publishedEvents }
}

function commandUsesStay(commandName: string, scope: string): boolean {
  if (scope.includes('stay') || scope.includes('checklist') || scope.includes('pre-arrival')) {
    return true
  }
  return commandName.includes('complete') || commandName.includes('uncomplete')
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
