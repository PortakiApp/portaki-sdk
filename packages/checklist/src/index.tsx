import { definePortakiModule } from '@portaki/module-sdk'
import ChecklistSection from './components/ChecklistSection'

export default definePortakiModule({
  id: 'checklist',
  label: { fr: 'Départ', en: 'Checkout' },
  icon: 'check-square',
  navSlot: 'section',
  visibleOnStatus: ['ACTIVE'],
  render: ({ stay, property, lang }) => {
    if (!stay) {
      return null
    }
    return <ChecklistSection stayId={stay.id} items={property.checklistItems} lang={lang} />
  },
})
