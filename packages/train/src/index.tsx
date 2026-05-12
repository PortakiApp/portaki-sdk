import { definePortakiModule } from '@portaki/module-sdk'
import TrainSection from './components/TrainSection'

export default definePortakiModule({
  id: 'train',
  version: '1.0.0',
  label: { fr: 'Horaires train', en: 'Train schedule' },
  description: {
    fr: 'Affiche les prochains départs SNCF depuis la gare la plus proche.',
    en: 'Shows upcoming SNCF departures from the nearest station.',
  },
  icon: 'train',
  navSlot: 'section',
  defaultNavLabel: { fr: 'Trains', en: 'Trains' },
  defaultNavIcon: 'train',
  visibleOnStatus: ['UPCOMING', 'ACTIVE'],

  config: {
    globalAlert: {
      type: 'info',
      message: {
        fr: 'Sans clé personnelle, Portaki utilise une clé mutualisée (cache 1h). Pour des données plus fraîches, obtenez votre clé gratuite.',
        en: 'Without a personal key, Portaki uses a shared key (1h cache). For fresher data, get your free key.',
      },
      helpUrl: 'https://navitia.io/register',
    },
    fields: [
      {
        key: 'navitia_api_key',
        label: { fr: 'Clé API Navitia (optionnelle)', en: 'Navitia API key (optional)' },
        description: {
          fr: 'Clé gratuite sur navitia.io. Sans clé, quota mutualisé avec cache agressif.',
          en: 'Free key at navitia.io. Without a key, shared quota with aggressive caching.',
        },
        type: 'secret',
        required: false,
        placeholder: {
          fr: 'xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx',
          en: 'xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx',
        },
      },
      {
        key: 'departure_station_name',
        label: { fr: 'Gare de départ', en: 'Departure station' },
        description: {
          fr: 'Nom de la gare la plus proche du logement.',
          en: 'Name of the nearest station.',
        },
        type: 'text',
        required: true,
        default: '',
        placeholder: { fr: 'Ex: Mandelieu-la-Napoule', en: 'Ex: Mandelieu-la-Napoule' },
      },
      {
        key: 'departure_station_code',
        label: { fr: 'Code Navitia de la gare', en: 'Navitia station code' },
        description: {
          fr: 'Code technique Navitia. Trouvez-le sur navitia.io.',
          en: 'Technical Navitia code. Find it at navitia.io.',
        },
        type: 'text',
        required: true,
        default: '',
        placeholder: {
          fr: 'Ex: stop_area:SNCF:87757005',
          en: 'Ex: stop_area:SNCF:87757005',
        },
        alert: {
          type: 'warning',
          message: {
            fr: 'Ce champ est requis pour que le module fonctionne.',
            en: 'This field is required for the module to work.',
          },
          helpUrl: 'https://docs.portaki.app/modules/train',
        },
      },
    ],
  },

  render: ({ stay, property, lang, config, track }) => (
    <TrainSection
      departureStationName={String(config.departure_station_name ?? '')}
      departureStationCode={
        String(config.departure_station_code ?? '') || property.trainStationCode || ''
      }
      lang={lang}
      onSearch={() => track({ type: 'action', label: 'search_train' })}
      onDepartureClick={(train) =>
        track({ type: 'click', label: 'departure_detail', metadata: { trainId: train.id } })
      }
    />
  ),
})
