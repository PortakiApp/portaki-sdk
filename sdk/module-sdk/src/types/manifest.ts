/** Onglet (ou autre surface) hôte déclaré dans portaki.module.json — rendu par portaki-web. */
export type ModuleHostSurfaceType = 'property-workspace-tab'

export type ModuleHostSurface = {
  type: ModuleHostSurfaceType
  pathSegment: string
  label: { fr: string; en: string }
  icon?: string
}

/** Sous-ensemble du manifeste utile au shell hôte (catalogue + surfaces). */
export type ModuleManifestHostHints = {
  id: string
  hostSurfaces?: ModuleHostSurface[]
}
