'use client'

import type { ModuleContext } from '@portaki/module-sdk'
import { definePortakiModule } from '@portaki/module-sdk'
import { useEffect, useState } from 'react'

type DailyForecast = {
  dates: string[]
  maxC: number[]
  minC: number[]
}

function numFromConfig(v: string | number | boolean | undefined): number | undefined {
  if (v === undefined || typeof v === 'boolean') return undefined
  const n = typeof v === 'number' ? v : Number(String(v).replace(',', '.'))
  return Number.isFinite(n) ? n : undefined
}

function WeatherBody({ ctx }: { ctx: ModuleContext }) {
  const { lang, config, property: propertyRaw } = ctx
  const property = propertyRaw as ModuleContext['property'] & { lat?: number; lng?: number }
  const cfgLat = numFromConfig(config.latitude)
  const cfgLng = numFromConfig(config.longitude)
  const lat = cfgLat ?? property.lat
  const lng = cfgLng ?? property.lng
  const rawDays = numFromConfig(config.forecast_days) ?? 3
  const forecastDays = Math.min(7, Math.max(1, Math.round(rawDays)))
  const locationLabel = String(config.location_label ?? '').trim()
  const intro = String(config.intro ?? '').trim()

  const [state, setState] = useState<'idle' | 'loading' | 'ok' | 'err' | 'nocoords'>('idle')
  const [data, setData] = useState<DailyForecast | null>(null)

  const coordsOk = lat != null && lng != null

  useEffect(() => {
    if (!coordsOk) {
      setState('nocoords')
      setData(null)
      return
    }
    let cancelled = false
    setState('loading')
    const url = new URL('https://api.open-meteo.com/v1/forecast')
    url.searchParams.set('latitude', String(lat))
    url.searchParams.set('longitude', String(lng))
    url.searchParams.set('daily', 'temperature_2m_max,temperature_2m_min')
    url.searchParams.set('forecast_days', String(forecastDays))
    url.searchParams.set('timezone', 'auto')
    fetch(url.toString())
      .then((r) => {
        if (!r.ok) throw new Error(String(r.status))
        return r.json() as Promise<{
          daily?: {
            time?: string[]
            temperature_2m_max?: number[]
            temperature_2m_min?: number[]
          }
        }>
      })
      .then((j) => {
        if (cancelled) return
        const d = j.daily
        const dates = d?.time ?? []
        const maxC = d?.temperature_2m_max ?? []
        const minC = d?.temperature_2m_min ?? []
        if (dates.length === 0) {
          setState('err')
          setData(null)
          return
        }
        setData({ dates, maxC, minC })
        setState('ok')
      })
      .catch(() => {
        if (!cancelled) {
          setState('err')
          setData(null)
        }
      })
    return () => {
      cancelled = true
    }
  }, [coordsOk, lat, lng, forecastDays])

  const title = lang === 'fr' ? 'Météo' : 'Weather'
  const area =
    locationLabel ||
    (lang === 'fr' ? 'Prévisions à proximité' : 'Forecast nearby')

  if (state === 'nocoords') {
    return (
      <section data-module="weather" className="space-y-2 text-sm">
        <h2 className="text-base font-semibold">{title}</h2>
        <p className="opacity-85">
          {lang === 'fr'
            ? 'Indiquez la latitude et la longitude dans le module, ou renseignez les coordonnées du logement dans Portaki.'
            : 'Set latitude/longitude in the module, or add property coordinates in Portaki.'}
        </p>
      </section>
    )
  }

  return (
    <section data-module="weather" className="space-y-3 text-sm">
      <h2 className="text-base font-semibold">{title}</h2>
      <p className="text-[12px] uppercase tracking-wide opacity-70">{area}</p>
      {intro ? <p className="whitespace-pre-wrap opacity-90">{intro}</p> : null}
      {state === 'loading' ? (
        <p className="opacity-75">{lang === 'fr' ? 'Chargement…' : 'Loading…'}</p>
      ) : null}
      {state === 'err' ? (
        <p className="text-red-700 dark:text-red-400">
          {lang === 'fr'
            ? 'Impossible de récupérer la météo pour le moment.'
            : 'Could not load weather right now.'}
        </p>
      ) : null}
      {state === 'ok' && data ? (
        <ul className="space-y-2">
          {data.dates.map((day, i) => {
            const tmax = data.maxC[i]
            const tmin = data.minC[i]
            const label =
              lang === 'fr'
                ? new Date(day + 'T12:00:00').toLocaleDateString('fr-FR', {
                    weekday: 'short',
                    day: 'numeric',
                    month: 'short',
                  })
                : new Date(day + 'T12:00:00').toLocaleDateString('en-GB', {
                    weekday: 'short',
                    day: 'numeric',
                    month: 'short',
                  })
            return (
              <li
                key={day}
                className="flex items-baseline justify-between gap-2 rounded-lg border border-black/10 px-3 py-2 dark:border-white/10"
              >
                <span className="font-medium capitalize">{label}</span>
                <span className="tabular-nums opacity-90">
                  {tmax != null && Number.isFinite(tmax) ? `${Math.round(tmax)}°` : '—'}
                  {tmin != null && Number.isFinite(tmin) ? (
                    <span className="opacity-70"> / {Math.round(tmin)}°</span>
                  ) : null}
                </span>
              </li>
            )
          })}
        </ul>
      ) : null}
      <p className="text-[11px] opacity-60">
        {lang === 'fr' ? 'Source : Open-Meteo' : 'Source: Open-Meteo'}
      </p>
    </section>
  )
}

export default definePortakiModule({
  id: 'weather',
  label: { fr: 'Météo', en: 'Weather' },
  description: {
    fr: 'Prévisions locales.',
    en: 'Local weather forecast.',
  },
  version: '1.0.0',
  icon: 'cloud-sun',
  navSlot: 'section',
  render: (ctx: ModuleContext) => <WeatherBody ctx={ctx} />,
})
