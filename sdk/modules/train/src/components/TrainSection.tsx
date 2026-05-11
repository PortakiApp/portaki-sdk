type TrainSectionProps = {
  departureCode?: string
  lang: 'fr' | 'en'
}

/** SNCF via Navitia — à brancher sur GET /api/v1/guest/.../train */
export default function TrainSection({ departureCode, lang }: TrainSectionProps) {
  return (
    <section data-module="train">
      <p>{lang === 'fr' ? 'Horaires trains' : 'Train schedules'}</p>
      {departureCode ? (
        <p className="text-sm opacity-80">{departureCode}</p>
      ) : (
        <p className="text-sm opacity-80">{lang === 'fr' ? 'Gare non configurée' : 'Station not set'}</p>
      )}
    </section>
  )
}
