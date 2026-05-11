import type { ModuleContext } from '@portaki/module-sdk'
import { definePortakiModule } from '@portaki/module-sdk'

export default definePortakiModule({
  id: 'appliances',
  label: { fr: 'Appareils', en: 'Appliances' },
  description: {
    fr: 'Guide des équipements.',
    en: 'Appliance guide.',
  },
  version: '1.0.0',
  icon: 'plug',
  navSlot: 'section',
  render: ({ lang }: ModuleContext) => (
    <section data-module="appliances">
      <p>{lang === 'fr' ? 'Guide appareils (TipTap)' : 'Appliance guide (TipTap)'}</p>
    </section>
  ),
})
