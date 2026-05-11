import type { ModuleContext } from '@portaki/module-sdk'
import { definePortakiModule } from '@portaki/module-sdk'
import ChecklistSection from './components/ChecklistSection'

export default definePortakiModule({
  id: 'checklist',
  label: { fr: 'Départ', en: 'Checkout' },
  description: {
    fr: 'Liste avant départ.',
    en: 'Checkout checklist.',
  },
  version: '1.0.0',
  icon: 'check-square',
  navSlot: 'section',
  visibleOnStatus: ['ACTIVE'],
  render: ({ stay, property, lang }: ModuleContext) => (
    <ChecklistSection stayId={stay.id} items={property.checklistItems} lang={lang} />
  ),
})
