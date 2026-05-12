'use client'

import type { HostModuleContext } from '@portaki/module-sdk'
import { hostModule } from '@portaki/module-sdk'

function IcalHostBody({ ctx }: { ctx: HostModuleContext }) {
  const summary = String(ctx.config.sync_summary ?? '').trim()
  return (
    <section data-module="ical-sync" className="space-y-3 text-sm">
      <h2 className="text-base font-semibold">
        {ctx.lang === 'fr' ? 'Calendriers' : 'Calendars'}
      </h2>
      <p className="leading-relaxed opacity-90">
        {ctx.lang === 'fr'
          ? 'Configurez les flux dans Portaki (logement → modules), puis lancez la synchronisation pour mettre à jour ce résumé.'
          : 'Configure feeds in Portaki (property → modules), then run sync to refresh this summary.'}
      </p>
      {summary ? (
        <pre className="max-h-48 overflow-auto whitespace-pre-wrap rounded-lg border border-black/10 bg-black/[0.02] p-3 text-[11px] leading-snug dark:border-white/10 dark:bg-white/[0.03]">
          {summary}
        </pre>
      ) : (
        <p className="text-[12px] opacity-70">
          {ctx.lang === 'fr' ? 'Aucune synchronisation pour l’instant.' : 'No sync yet.'}
        </p>
      )}
    </section>
  )
}

export default hostModule('ical-sync')
  .label('Calendriers (iCal / Airbnb)', 'Calendars (iCal / Airbnb)')
  .description(
    'Synchronisation des flux iCal (Airbnb, etc.) pour le suivi interne.',
    'iCal feed sync (Airbnb, etc.) for internal tracking.',
  )
  .icon('calendar-range')
  .version('1.0.0')
  .navSlot('section')
  .hostRender((ctx) => <IcalHostBody ctx={ctx} />)
  .build()
