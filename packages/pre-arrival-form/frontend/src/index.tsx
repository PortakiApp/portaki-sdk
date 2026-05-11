import type { ModuleContext } from '@portaki/module-sdk'
import { definePortakiModule } from '@portaki/module-sdk'
import PreArrivalForm from './components/PreArrivalForm'

export default definePortakiModule({
  id: 'pre-arrival-form',
  label: { fr: 'Avant votre arrivée', en: 'Before your arrival' },
  description: {
    fr: 'Formulaire avant arrivée.',
    en: 'Pre-arrival form.',
  },
  version: '1.0.0',
  icon: 'clipboard-list',
  navSlot: 'section',
  visibleOnStatus: ['PRE_ARRIVAL', 'UPCOMING'],
  render: ({ stay, lang }: ModuleContext) => (
    <PreArrivalForm stayId={stay.id} lang={lang} />
  ),
})
