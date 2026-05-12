import type { ModuleContext } from '@portaki/module-sdk'
import { definePortakiModule } from '@portaki/module-sdk'

type Localized = { fr: string; en: string }

type Facility = {
  id: string
  title: Localized
  lines: Localized[]
  note?: Partial<Localized>
}

function parseFacilities(raw: string | undefined): Facility[] {
  if (!raw?.trim()) return []
  try {
    const data = JSON.parse(raw) as unknown
    if (!Array.isArray(data)) return []
    return data.filter((x): x is Facility => x != null && typeof x === 'object' && 'id' in x && 'title' in x)
  } catch {
    return []
  }
}

export default definePortakiModule({
  id: 'facility-hours',
  label: { fr: 'Horaires & accès', en: 'Hours & access' },
  description: {
    fr: 'Piscine, spa, équipements partagés.',
    en: 'Pool, spa, shared amenities.',
  },
  version: '1.0.0',
  icon: 'waves',
  navSlot: 'section',
  render: ({ lang, config }: ModuleContext) => {
    const facilities = parseFacilities(String(config.facilities_json ?? ''))
    const general = String(config.general_note ?? '').trim()

    if (facilities.length === 0 && !general) {
      return (
        <section data-module="facility-hours" className="text-sm opacity-80">
          <p className="font-medium">{lang === 'fr' ? 'Horaires' : 'Hours'}</p>
          <p>
            {lang === 'fr'
              ? 'À configurer par logement (JSON) dans l’espace hôte.'
              : 'Configure per property (JSON) in the host area.'}
          </p>
        </section>
      )
    }

    return (
      <section data-module="facility-hours" className="space-y-4 text-sm">
        <h2 className="text-base font-semibold">
          {lang === 'fr' ? 'Horaires & accès' : 'Hours & access'}
        </h2>
        {general ? (
          <p className="rounded-lg border border-black/10 bg-white/50 px-3 py-2 whitespace-pre-wrap dark:bg-black/20">
            {general}
          </p>
        ) : null}
        <ul className="space-y-4">
          {facilities.map((f) => {
            const title = lang === 'en' ? f.title.en || f.title.fr : f.title.fr || f.title.en
            const noteRaw = lang === 'en' ? f.note?.en ?? f.note?.fr : f.note?.fr ?? f.note?.en
            return (
              <li key={f.id} className="rounded-lg border border-black/10 px-3 py-2">
                <p className="font-medium">{title}</p>
                <ul className="mt-2 list-disc space-y-1 pl-5 text-[13px] opacity-90">
                  {f.lines.map((line, i) => {
                    const t = lang === 'en' ? line.en || line.fr : line.fr || line.en
                    return <li key={i}>{t}</li>
                  })}
                </ul>
                {noteRaw ? (
                  <p className="mt-2 text-[12px] text-amber-900/90 dark:text-amber-200/90">{noteRaw}</p>
                ) : null}
              </li>
            )
          })}
        </ul>
      </section>
    )
  },
})
