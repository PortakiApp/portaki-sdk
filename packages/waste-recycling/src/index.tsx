import type { ModuleContext } from '@portaki/module-sdk'
import { definePortakiModule } from '@portaki/module-sdk'

type Localized = { fr: string; en: string }

type BinRow = {
  id: string
  title: Localized
  items: Localized[]
}

function parseBins(raw: string | undefined): BinRow[] {
  if (!raw?.trim()) return []
  try {
    const data = JSON.parse(raw) as unknown
    if (!Array.isArray(data)) return []
    return data.filter((x): x is BinRow => x != null && typeof x === 'object' && 'id' in x && 'title' in x)
  } catch {
    return []
  }
}

export default definePortakiModule({
  id: 'waste-recycling',
  label: { fr: 'Tri & déchets', en: 'Recycling & waste' },
  description: {
    fr: 'Bacs et jours de collecte.',
    en: 'Bins and collection days.',
  },
  version: '1.0.0',
  icon: 'recycle',
  navSlot: 'section',
  render: ({ lang, config }: ModuleContext) => {
    const bins = parseBins(String(config.bins_json ?? ''))
    const schedule = String(config.collection_schedule ?? '').trim()

    if (bins.length === 0 && !schedule) {
      return (
        <section data-module="waste-recycling" className="text-sm opacity-80">
          <p className="font-medium">{lang === 'fr' ? 'Tri' : 'Recycling'}</p>
          <p>
            {lang === 'fr'
              ? 'À configurer (JSON des bacs) dans l’espace hôte.'
              : 'Configure bin JSON from the host area.'}
          </p>
        </section>
      )
    }

    return (
      <section data-module="waste-recycling" className="space-y-4 text-sm">
        <h2 className="text-base font-semibold">
          {lang === 'fr' ? 'Tri & déchets' : 'Recycling & waste'}
        </h2>
        <ul className="space-y-3">
          {bins.map((b) => {
            const title = lang === 'en' ? b.title.en || b.title.fr : b.title.fr || b.title.en
            return (
              <li key={b.id} className="rounded-lg border border-black/10 px-3 py-2">
                <p className="font-medium">{title}</p>
                <ul className="mt-2 list-disc space-y-1 pl-5 text-[13px]">
                  {(b.items ?? []).map((it, i) => {
                    const t = lang === 'en' ? it.en || it.fr : it.fr || it.en
                    return <li key={i}>{t}</li>
                  })}
                </ul>
              </li>
            )
          })}
        </ul>
        {schedule ? (
          <div className="rounded-lg border border-black/10 bg-white/50 px-3 py-2 whitespace-pre-wrap dark:bg-black/20">
            <p className="text-[11px] font-medium uppercase tracking-wide opacity-70">
              {lang === 'fr' ? 'Collecte' : 'Collection'}
            </p>
            <p className="mt-1">{schedule}</p>
          </div>
        ) : null}
      </section>
    )
  },
})
