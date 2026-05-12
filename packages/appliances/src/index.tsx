import type { ModuleContext } from '@portaki/module-sdk'
import { definePortakiModule } from '@portaki/module-sdk'

type Localized = { fr: string; en: string }

type DeviceRow = {
  id: string
  title: Localized
  tips?: Partial<Localized>
  manualUrl?: string
  icon?: string
}

function parseDevices(raw: string | undefined): DeviceRow[] {
  if (!raw?.trim()) return []
  try {
    const data = JSON.parse(raw) as unknown
    if (!Array.isArray(data)) return []
    return data.filter((x): x is DeviceRow => x != null && typeof x === 'object' && 'id' in x && 'title' in x)
  } catch {
    return []
  }
}

export default definePortakiModule({
  id: 'appliances',
  label: { fr: 'Appareils', en: 'Appliances' },
  description: {
    fr: 'Guide des équipements.',
    en: 'Appliance guide.',
  },
  version: '1.0.0',
  icon: 'plug',
  navSlot: 'section',
  render: ({ lang, config }: ModuleContext) => {
    const devices = parseDevices(String(config.devices_json ?? ''))
    const safety = String(config.safety_notice ?? '').trim()

    if (devices.length === 0 && !safety) {
      return (
        <section data-module="appliances" className="space-y-2 text-sm opacity-80">
          <p className="font-medium">
            {lang === 'fr' ? 'Guide appareils' : 'Appliance guide'}
          </p>
          <p>
            {lang === 'fr'
              ? 'L’hôte configure la liste (JSON) depuis l’espace hôte.'
              : 'The host configures the list (JSON) from the host area.'}
          </p>
        </section>
      )
    }

    return (
      <section data-module="appliances" className="space-y-4 text-sm">
        <h2 className="text-base font-semibold">
          {lang === 'fr' ? 'Appareils & consignes' : 'Appliances & tips'}
        </h2>
        {safety ? (
          <div className="rounded-lg border border-amber-500/40 bg-amber-500/10 px-3 py-2 text-[13px] leading-snug">
            {safety}
          </div>
        ) : null}
        <ul className="space-y-3">
          {devices.map((d) => {
            const title = lang === 'en' ? d.title.en || d.title.fr : d.title.fr || d.title.en
            const tipsRaw = lang === 'en' ? d.tips?.en ?? d.tips?.fr : d.tips?.fr ?? d.tips?.en
            const manual = (d.manualUrl ?? '').trim()
            return (
              <li
                key={d.id}
                className="rounded-lg border border-black/10 bg-white/60 px-3 py-2 dark:bg-black/20"
              >
                <p className="font-medium">{title}</p>
                {tipsRaw ? <p className="mt-1 text-[13px] opacity-85 whitespace-pre-wrap">{tipsRaw}</p> : null}
                {manual ? (
                  <a
                    href={manual}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="mt-2 inline-block text-[13px] font-medium text-sky-700 underline dark:text-sky-400"
                  >
                    {lang === 'fr' ? 'Notice / PDF' : 'Manual / PDF'}
                  </a>
                ) : null}
              </li>
            )
          })}
        </ul>
      </section>
    )
  },
})
