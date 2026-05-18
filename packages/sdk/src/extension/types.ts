/** Context for host-only module actions (`sync`, …). */
export type HostExtensionContext = {
  readonly tenantId: string
  readonly propertyId: string
  readonly moduleId: string
}

export type HostModuleRunResult = {
  readonly ok: boolean
  readonly succeeded: number
  readonly failed: number
  readonly itemsTotal: number
  readonly summary: string
  readonly updatedPlainConfigJson: string
}

export type HostActionHandler = (
  ctx: HostExtensionContext,
  plainConfig: Record<string, unknown>,
) => Promise<HostModuleRunResult> | HostModuleRunResult

export type EventHandlerContext = {
  readonly tenantId: string
  readonly propertyId: string
  readonly moduleId: string
  readonly stayId: string | null
  readonly config: Record<string, unknown>
}

export type EventHandler = (
  ctx: EventHandlerContext,
  eventData: Record<string, unknown>,
) => Promise<void> | void

export type ModuleExtensionDefinition = {
  readonly hostActions?: Readonly<Record<string, HostActionHandler>>
  readonly eventHandlers?: Readonly<Record<string, EventHandler>>
}
