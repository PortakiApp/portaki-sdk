import type { ModuleContext } from '@portaki/module-sdk'
import { definePortakiModule } from '@portaki/module-sdk'

export default definePortakiModule({
  id: 'wifi-guest',
  label: { fr: 'Wi‑Fi invité', en: 'Guest Wi‑Fi' },
  description: {
    fr: 'SSID et mot de passe pour le réseau invité.',
    en: 'SSID and password for the guest network.',
  },
  version: '1.0.0',
  icon: 'wifi',
  navSlot: 'section',
  render: ({ lang, config }: ModuleContext) => {
    const ssid = String(config.ssid ?? '').trim()
    const password = String(config.password ?? '').trim()
    const band = String(config.band_hint ?? '').trim()
    const steps = String(config.connection_steps ?? '').trim()

    if (!ssid && !password) {
      return (
        <section data-module="wifi-guest" className="text-sm opacity-80">
          <p className="font-medium">Wi‑Fi</p>
          <p>
            {lang === 'fr'
              ? 'Renseignez le SSID et le mot de passe dans le configurateur hôte.'
              : 'Set SSID and password in the host configurator.'}
          </p>
        </section>
      )
    }

    return (
      <section data-module="wifi-guest" className="space-y-3 text-sm">
        <h2 className="text-base font-semibold">Wi‑Fi</h2>
        {ssid ? (
          <div>
            <p className="text-[11px] uppercase tracking-wide opacity-70">SSID</p>
            <p className="font-mono text-base font-medium">{ssid}</p>
          </div>
        ) : null}
        {password ? (
          <div>
            <p className="text-[11px] uppercase tracking-wide opacity-70">
              {lang === 'fr' ? 'Mot de passe' : 'Password'}
            </p>
            <p className="font-mono text-base font-medium break-all">{password}</p>
          </div>
        ) : null}
        {band ? <p className="text-[13px] opacity-90">{band}</p> : null}
        {steps ? (
          <p className="whitespace-pre-wrap rounded-lg border border-black/10 bg-white/50 px-3 py-2 dark:bg-black/20">
            {steps}
          </p>
        ) : null}
      </section>
    )
  },
})
