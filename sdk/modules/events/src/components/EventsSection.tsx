type EventsSectionProps = {
  propertyId: string
  lang: 'fr' | 'en'
}

export default function EventsSection({ propertyId, lang }: EventsSectionProps) {
  return (
    <section data-module="events">
      <p>{lang === 'fr' ? 'Événements' : 'Events'}</p>
      <p className="text-sm opacity-80">{propertyId}</p>
    </section>
  )
}
