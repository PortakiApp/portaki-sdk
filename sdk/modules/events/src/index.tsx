import { definePortakiModule } from '@portakiapp/module-sdk'
import EventsSection from './components/EventsSection'

export default definePortakiModule({
  id: 'events',
  label: { fr: 'Événements', en: 'Events' },
  icon: 'calendar',
  navSlot: 'section',
  mapOverlay: true,
  render: ({ property, lang }) => <EventsSection propertyId={property.id} lang={lang} />,
  mapMarkers: async ({ property, lang }) => {
    void property
    void lang
    return []
  },
})
