type Item = { id: string; labelFr: string; labelEn: string }

type ChecklistSectionProps = {
  stayId: string
  items?: readonly Item[]
  lang: 'fr' | 'en'
}

export default function ChecklistSection({ stayId, items, lang }: ChecklistSectionProps) {
  return (
    <section data-module="checklist">
      <p>{lang === 'fr' ? 'Checklist départ' : 'Checkout checklist'}</p>
      <p className="text-sm opacity-80">stay: {stayId}</p>
      <ul className="list-disc pl-5">
        {(items ?? []).map((i) => (
          <li key={i.id}>{lang === 'fr' ? i.labelFr : i.labelEn}</li>
        ))}
      </ul>
    </section>
  )
}
