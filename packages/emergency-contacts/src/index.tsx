import type { ModuleContext } from '@portaki/module-sdk'
import { definePortakiModule } from '@portaki/module-sdk'

type Localized = { fr: string; en: string }

type ContactRow = {
  id: string
  label: Localized
  phone: string
  note?: Partial<Localized>
  category?: string
}

function parseContacts(raw: string | undefined): ContactRow[] {
  if (!raw?.trim()) return []
  try {
    const data = JSON.parse(raw) as unknown
    if (!Array.isArray(data)) return []
    return data.filter(
      (x): x is ContactRow =>
        x != null &&
        typeof x === 'object' &&
        'id' in x &&
        typeof (x as ContactRow).id === 'string' &&
        'label' in x &&
        typeof (x as ContactRow).label === 'object' &&
        (x as ContactRow).label != null &&
        'phone' in x &&
        typeof (x as ContactRow).phone === 'string',
    )
  } catch {
    return []
  }
}

export default definePortakiModule({
  id: 'emergency-contacts',
  label: { fr: 'Urgences & utiles', en: 'Emergency & useful' },
  description: {
    fr: 'Numéros utiles et ligne hôte.',
    en: 'Useful numbers and host line.',
  },
  version: '1.0.0',
  icon: 'phone',
  navSlot: 'section',
  render: ({ lang, config }: ModuleContext) => {
    const rows = parseContacts(String(config.contacts_json ?? ''))
    const hostPhone = String(config.host_visible_phone ?? '').trim()

    if (rows.length === 0 && !hostPhone) {
      return (
        <section data-module="emergency-contacts" className="text-sm opacity-80">
          <p className="font-medium">{lang === 'fr' ? 'Contacts' : 'Contacts'}</p>
          <p>
            {lang === 'fr'
              ? 'Ajoutez les contacts (JSON) depuis le configurateur hôte.'
              : 'Add contacts (JSON) from the host configurator.'}
          </p>
        </section>
      )
    }

    return (
      <section data-module="emergency-contacts" className="space-y-4 text-sm">
        <h2 className="text-base font-semibold">
          {lang === 'fr' ? 'Urgences & numéros utiles' : 'Emergency & useful numbers'}
        </h2>
        <p className="rounded-lg border border-amber-200/80 bg-amber-50/80 px-3 py-2 text-[13px] dark:border-amber-900/50 dark:bg-amber-950/40">
          {lang === 'fr'
            ? 'En cas d’urgence vitale, composez le 112 ou le 15 (SAMU) selon les consignes locales.'
            : 'For life-threatening emergencies, dial 112 or your local emergency number.'}
        </p>
        {hostPhone ? (
          <div className="rounded-lg border border-black/10 px-3 py-2 dark:border-white/10">
            <p className="text-[11px] uppercase tracking-wide opacity-70">
              {lang === 'fr' ? 'Hôte' : 'Host'}
            </p>
            <a href={`tel:${hostPhone.replace(/\s+/g, '')}`} className="text-lg font-semibold text-sky-700 underline dark:text-sky-400">
              {hostPhone}
            </a>
          </div>
        ) : null}
        {rows.length > 0 ? (
          <ul className="space-y-3">
            {rows.map((r) => {
              const label =
                lang === 'en' ? r.label.en || r.label.fr : r.label.fr || r.label.en
              const noteRaw =
                lang === 'en' ? r.note?.en ?? r.note?.fr : r.note?.fr ?? r.note?.en
              const note = noteRaw?.trim()
              const tel = r.phone.replace(/\s+/g, '')
              const cat = (r.category ?? '').trim()
              return (
                <li
                  key={r.id}
                  className="flex flex-col gap-1 rounded-lg border border-black/10 px-3 py-2 dark:border-white/10"
                >
                  {cat ? (
                    <span className="text-[10px] font-medium uppercase tracking-wide opacity-60">
                      {cat}
                    </span>
                  ) : null}
                  <div className="flex flex-wrap items-baseline justify-between gap-2">
                    <span className="font-medium">{label}</span>
                    <a
                      href={`tel:${tel}`}
                      className="shrink-0 font-mono text-base text-sky-700 underline dark:text-sky-400"
                    >
                      {r.phone}
                    </a>
                  </div>
                  {note ? <p className="text-[13px] opacity-85 whitespace-pre-wrap">{note}</p> : null}
                </li>
              )
            })}
          </ul>
        ) : null}
      </section>
    )
  },
})
