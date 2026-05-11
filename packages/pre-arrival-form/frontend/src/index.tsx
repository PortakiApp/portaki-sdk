import { definePortakiModule } from '@portaki/module-sdk'
import PreArrivalForm from './components/PreArrivalForm'

export default definePortakiModule({
  id: 'pre-arrival-form',
  label: { fr: 'Avant votre arrivée', en: 'Before your arrival' },
  icon: 'clipboard-list',
  navSlot: 'section',
  visibleOnStatus: ['PRE_ARRIVAL', 'UPCOMING'],
  render: ({ stay, lang }) => {
    if (!stay) {
      return null
    }
    return <PreArrivalForm stayId={stay.id} lang={lang} />
  },
})
