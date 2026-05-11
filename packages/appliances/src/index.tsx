import { definePortakiModule } from '@portaki/module-sdk'

export default definePortakiModule({
  id: 'appliances',
  label: { fr: 'Appareils', en: 'Appliances' },
  icon: 'plug',
  navSlot: 'section',
  render: ({ lang }) => (
    <section data-module="appliances">
      <p>{lang === 'fr' ? 'Guide appareils (TipTap)' : 'Appliance guide (TipTap)'}</p>
    </section>
  ),
})
