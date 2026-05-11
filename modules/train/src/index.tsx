import { definePortakiModule } from '@portakiapp/module-sdk'
import TrainSection from './components/TrainSection'

export default definePortakiModule({
  id: 'train',
  label: { fr: 'Trains', en: 'Trains' },
  icon: 'train',
  navSlot: 'section',
  render: ({ property, lang }) => (
    <TrainSection departureCode={property.trainStationCode} lang={lang} />
  ),
})
