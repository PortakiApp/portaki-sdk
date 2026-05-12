import type { ModuleContext } from '@portaki/module-sdk'
import { definePortakiModule, ExternalLink } from '@portaki/module-sdk'

type Localized = { fr: string; en: string }

type StepKind = 'parking' | 'door' | 'elevator' | 'other'

type AccessStep = {
  id: string
  kind?: StepKind
  title: Localized
  detail?: Partial<Localized>
}

function parseSteps(raw: string | undefined): AccessStep[] {
  if (!raw?.trim()) return []
  try {
    const data = JSON.parse(raw) as unknown
    if (!Array.isArray(data)) return []
    return data.filter(
      (x): x is AccessStep =>
        x != null &&
        typeof x === 'object' &&
        'id' in x &&
        typeof (x as AccessStep).id === 'string' &&
        'title' in x &&
        typeof (x as AccessStep).title === 'object' &&
        (x as AccessStep).title != null,
    )
  } catch {
    return []
  }
}

function kindLabel(kind: StepKind | undefined, lang: 'fr' | 'en'): string {
  if (lang === 'en') {
    switch (kind) {
      case 'parking':
        return 'Parking'
      case 'door':
        return 'Door'
      case 'elevator':
        return 'Lift'
      default:
        return 'Step'
    }
  }
  switch (kind) {
    case 'parking':
      return 'Parking'
    case 'door':
      return 'Porte'
    case 'elevator':
      return 'Ascenseur'
    default:
      return 'Étape'
  }
}

export default definePortakiModule({
  id: 'access-guide',
  label: { fr: 'Accès & parking', en: 'Access & parking' },
  description: {
    fr: 'Étapes jusqu’au logement.',
    en: 'Steps to reach the stay.',
  },
  version: '1.0.0',
  icon: 'car-front',
  navSlot: 'section',
  render: ({ lang, config }: ModuleContext) => {
    const steps = parseSteps(String(config.steps_json ?? ''))
    const mapUrl = String(config.parking_map_url ?? '').trim()
    const videoUrl = String(config.arrival_video_url ?? '').trim()
    const globalNote = String(config.global_note ?? '').trim()

    if (steps.length === 0 && !mapUrl && !videoUrl && !globalNote) {
      return (
        <section data-module="access-guide" className="text-sm opacity-80">
          <p className="font-medium">{lang === 'fr' ? 'Accès' : 'Access'}</p>
          <p>
            {lang === 'fr'
              ? 'Ajoutez les étapes (JSON) depuis le configurateur hôte.'
              : 'Add access steps (JSON) from the host configurator.'}
          </p>
        </section>
      )
    }

    return (
      <section data-module="access-guide" className="space-y-4 text-sm">
        <h2 className="text-base font-semibold">
          {lang === 'fr' ? 'Accès & parking' : 'Access & parking'}
        </h2>
        {globalNote ? (
          <p className="whitespace-pre-wrap rounded-lg border border-black/10 bg-white/50 px-3 py-2 dark:bg-black/20">
            {globalNote}
          </p>
        ) : null}
        {mapUrl ? (
          <div>
            <p className="text-[11px] uppercase tracking-wide opacity-70">
              {lang === 'fr' ? 'Plan / carte' : 'Map'}
            </p>
            <ExternalLink href={mapUrl}>
              {lang === 'fr' ? 'Ouvrir le lien' : 'Open link'}
            </ExternalLink>
          </div>
        ) : null}
        {videoUrl ? (
          <div>
            <p className="text-[11px] uppercase tracking-wide opacity-70">
              {lang === 'fr' ? 'Vidéo' : 'Video'}
            </p>
            <ExternalLink href={videoUrl}>
              {lang === 'fr' ? 'Voir la vidéo d’arrivée' : 'Watch arrival video'}
            </ExternalLink>
          </div>
        ) : null}
        {steps.length > 0 ? (
          <ol className="list-decimal space-y-3 pl-4">
            {steps.map((s) => {
              const title =
                lang === 'en' ? s.title.en || s.title.fr : s.title.fr || s.title.en
              const detailRaw =
                lang === 'en' ? s.detail?.en ?? s.detail?.fr : s.detail?.fr ?? s.detail?.en
              const detail = detailRaw?.trim()
              const badge = kindLabel(s.kind, lang)
              return (
                <li key={s.id} className="space-y-1">
                  <div className="flex flex-wrap items-center gap-2">
                    <span className="rounded bg-black/5 px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide dark:bg-white/10">
                      {badge}
                    </span>
                    <span className="font-medium">{title}</span>
                  </div>
                  {detail ? <p className="whitespace-pre-wrap opacity-90">{detail}</p> : null}
                </li>
              )
            })}
          </ol>
        ) : null}
      </section>
    )
  },
})
