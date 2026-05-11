import { definePortakiModule } from '@portaki/module-sdk'

export default definePortakiModule({
  id: 'rules',
  label: { fr: 'Règlement', en: 'House rules' },
  icon: 'scale',
  navSlot: 'section',
  render: ({ lang }) => (
    <section data-module="rules">
      <p>{lang === 'fr' ? 'Règlement intérieur (TipTap)' : 'House rules (TipTap)'}</p>
    </section>
  ),
})
