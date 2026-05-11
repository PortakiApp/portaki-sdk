export type TrainDeparture = { id: string }

type TrainSectionProps = {
  departureStationName: string
  departureStationCode: string
  lang: 'fr' | 'en'
  onSearch?: () => void
  onDepartureClick?: (train: TrainDeparture) => void
}

/** SNCF via Navitia — branché sur GET /api/v1/guest/.../train */
export default function TrainSection({
  departureStationName,
  departureStationCode,
  lang,
  onSearch,
  onDepartureClick,
}: TrainSectionProps) {
  const configured = Boolean(departureStationCode?.trim())

  return (
    <section data-module="train">
      <p className="font-medium">{lang === 'fr' ? 'Horaires trains' : 'Train schedules'}</p>
      {departureStationName ? (
        <p className="mt-1 text-sm opacity-80">{departureStationName}</p>
      ) : null}
      {configured ? (
        <p className="mt-2 text-sm opacity-80">{departureStationCode}</p>
      ) : (
        <p className="mt-2 text-sm opacity-80">
          {lang === 'fr' ? 'Gare non configurée' : 'Station not set'}
        </p>
      )}
      <div className="mt-4 flex flex-wrap gap-2">
        <button
          type="button"
          className="rounded-lg border border-black/10 px-3 py-1.5 text-sm"
          onClick={() => {
            onSearch?.()
          }}
        >
          {lang === 'fr' ? 'Actualiser' : 'Refresh'}
        </button>
        <button
          type="button"
          className="rounded-lg border border-black/10 px-3 py-1.5 text-sm"
          onClick={() => onDepartureClick?.({ id: 'demo' })}
        >
          {lang === 'fr' ? 'Exemple détail' : 'Sample detail'}
        </button>
      </div>
    </section>
  )
}
