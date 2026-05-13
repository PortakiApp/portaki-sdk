import type { LangCode } from '../types/module'
import type { ModuleConfigAlert as ModuleConfigAlertSpec } from '../types/config'

export interface ModuleConfigAlertProps {
  alert: ModuleConfigAlertSpec
  lang: LangCode
}

const ICON = {
  info: 'ⓘ',
  warning: '⚠',
  error: '!',
  success: '✓',
} as const

export function ModuleConfigAlert({ alert, lang }: ModuleConfigAlertProps) {
  const colors = {
    info: {
      bg: 'rgba(43,127,191,0.08)',
      border: '#2B7FBF',
      text: '#1e40af',
    },
    warning: {
      bg: 'rgba(245,158,11,0.08)',
      border: '#F59E0B',
      text: '#92400e',
    },
    error: {
      bg: 'rgba(239,68,68,0.08)',
      border: '#EF4444',
      text: '#991b1b',
    },
    success: {
      bg: 'rgba(132,204,22,0.08)',
      border: '#84CC16',
      text: '#3f6212',
    },
  }

  const c = colors[alert.type]

  return (
    <div
      style={{
        background: c.bg,
        border: `1px solid ${c.border}`,
        borderRadius: '8px',
        padding: '10px 12px',
        display: 'flex',
        gap: '8px',
        alignItems: 'flex-start',
        marginTop: '8px',
      }}
    >
      <span style={{ flexShrink: 0, marginTop: '2px', fontSize: '14px' }} aria-hidden>
        {ICON[alert.type]}
      </span>
      <div>
        <p style={{ fontSize: '13px', color: c.text, lineHeight: 1.5 }}>{alert.message[lang]}</p>
        {alert.helpUrl ? (
          <a
            href={alert.helpUrl}
            target="_blank"
            rel="noopener noreferrer"
            style={{
              fontSize: '12px',
              color: c.border,
              textDecoration: 'underline',
              marginTop: '4px',
              display: 'inline-block',
            }}
          >
            En savoir plus →
          </a>
        ) : null}
      </div>
    </div>
  )
}
