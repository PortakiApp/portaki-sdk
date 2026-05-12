import type { ModuleContext } from '@portaki/module-sdk'
import { definePortakiModule } from '@portaki/module-sdk'

type Localized = { fr: string; en: string }

type Spot = {
  id: string
  title: Localized
  url?: string
  category?: string
  note?: Partial<Localized>
}

function parseSpots(raw: string | undefined): Spot[] {
  if (!raw?.trim()) return []
  try {
    const data = JSON.parse(raw) as unknown
    if (!Array.isArray(data)) return []
    return data.filter((x): x is Spot => x != null && typeof x === 'object' && 'id' in x && 'title' in x)
  } catch {
    return []
  }
}

export default definePortakiModule({
  id: 'local-guide',
  label: { fr: 'Bons plans', en: 'Local picks' },
  description: {
    fr: 'Adresses et liens utiles autour du logement.',
    en: 'Useful spots and links near the stay.',
  },
  version: '1.0.0',
  icon: 'map-pin',
  navSlot: 'section',
  render: ({ lang, config }: ModuleContext) => {
    const spots = parseSpots(String(config.spots_json ?? ''))
    const disclaimer = String(config.disclaimer ?? '').trim()

    if (spots.length === 0 && !disclaimer) {
      return (
        <section data-module="local-guide" className="text-sm opacity-80">
          <p className="font-medium">{lang === 'fr' ? 'Autour de vous' : 'Around you'}</p>
          <p>
            {lang === 'fr'
              ? 'Ajoutez des lieux (JSON) depuis le configurateur hôte.'
              : 'Add places (JSON) from the host configurator.'}
          </p>
        </section>
      )
    }

    return (
      <section data-module="local-guide" className="space-y-4 text-sm">
        <h2 className="text-base font-semibold">
          {lang === 'fr' ? 'Bons plans du coin' : 'Local picks'}
        </h2>
        {disclaimer ? (
          <p className="text-[12px] opacity-80 whitespace-pre-wrap">{disclaimer}</p>
        ) : null}
        <ul className="space-y-2">
          {spots.map((s) => {
            const title = lang === 'en' ? s.title.en || s.title.fr : s.title.fr || s.title.en
            const noteRaw = lang === 'en' ? s.note?.en ?? s.note?.fr : s.note?.fr ?? s.note?.en
            const href = (s.url ?? '').trim()
            return (
              <li
                key={s.id}
                className="flex flex-col gap-1 rounded-lg border border-black/10 px-3 py-2"
              >
                <div className="flex flex-wrap items-baseline justify-between gap-2">
                  {href ? (
                    <a
                      href={href}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="font-medium text-sky-700 underline dark:text-sky-400"
                    >
                      {title}
                    </a>
                  ) : (
                    <span className="font-medium">{title}</span>
                  )}
                  {s.category ? (
                    <span className="text-[11px] uppercase tracking-wide opacity-60">{s.category}</span>
                  ) : null}
                </div>
                {noteRaw ? <p className="text-[13px] opacity-85">{noteRaw}</p> : null}
              </li>
            )
          })}
        </ul>
      </section>
    )
  },
})
