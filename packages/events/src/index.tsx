import type { ModuleContext } from '@portaki/module-sdk'
import { definePortakiModule } from '@portaki/module-sdk'
import EventsSection from './components/EventsSection'

export default definePortakiModule({
  id: 'events',
  label: { fr: 'Événements', en: 'Events' },
  description: {
    fr: 'Faits marquants et sorties autour du logement.',
    en: 'Highlights and nearby happenings.',
  },
  version: '1.0.0',
  icon: 'calendar',
  navSlot: 'section',
  mapOverlay: true,
  render: ({ property, lang }: ModuleContext) => (
    <EventsSection propertyId={property.id} lang={lang} />
  ),
  mapMarkers: async ({ property, lang }) => {
    void property
    void lang
    return []
  },
})
