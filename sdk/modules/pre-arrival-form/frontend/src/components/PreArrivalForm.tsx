type PreArrivalFormProps = {
  stayId: string
  lang: 'fr' | 'en'
}

export default function PreArrivalForm({ stayId, lang }: PreArrivalFormProps) {
  return (
    <section data-module="pre-arrival-form">
      <p>{lang === 'fr' ? 'Avant votre arrivée' : 'Before your arrival'}</p>
      <p className="text-sm opacity-80">{stayId}</p>
    </section>
  )
}
