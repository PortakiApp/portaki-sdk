/** Onglet (ou autre surface) hôte déclaré dans portaki.module.json — rendu par portaki-web. */
export type ModuleHostSurfaceType = 'property-workspace-tab'

export type ModuleHostSurface = {
  type: ModuleHostSurfaceType
  pathSegment: string
  label: { fr: string; en: string }
  icon?: string
}

export type ModuleEmailTriggerType =
  | 'relativeToCheckIn'
  | 'relativeToCheckOut'
  | 'onStayCreated'

export type ModuleEmailTrigger = {
  type: ModuleEmailTriggerType
  /** ISO-8601 duration, e.g. `-P1D` */
  offset?: string
  /** Local send window `HH:mm` */
  atLocalTime?: string
}

export type ModuleEmailSkipWhen =
  | 'guest.email.missing'
  | 'module.disabled'
  | 'stay.preArrivalCompleted'
  | 'stay.cancelled'

export type ModuleEmailDeclaration = {
  id: string
  description: { fr: string; en: string }
  audience: 'guest' | 'host'
  requiresGuestEmail?: boolean
  trigger: ModuleEmailTrigger
  skipWhen?: ModuleEmailSkipWhen[]
}

export type ModuleGuestActionKind = 'open-module'

export type ModuleGuestActionDeclaration = {
  id: string
  description: { fr: string; en: string }
  kind: ModuleGuestActionKind
  moduleId: string
}

/** Sous-ensemble du manifeste utile au shell hôte (catalogue + surfaces). */
export type ModuleManifestHostHints = {
  id: string
  hostSurfaces?: ModuleHostSurface[]
  emails?: ModuleEmailDeclaration[]
  guestActions?: ModuleGuestActionDeclaration[]
}
