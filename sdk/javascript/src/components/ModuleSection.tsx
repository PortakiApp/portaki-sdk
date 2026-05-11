import type { ReactNode } from 'react'

export interface ModuleSectionProps {
  title: string
  children: ReactNode
  className?: string
}

export function ModuleSection({ title, children, className }: ModuleSectionProps) {
  return (
    <section className={className} style={{ padding: '24px 0' }}>
      <h2
        style={{
          fontFamily: 'var(--brand-font-heading, var(--font-cal))',
          fontSize: '22px',
          fontWeight: 600,
          color: 'var(--brand-color, var(--text))',
          marginBottom: '16px',
          letterSpacing: '-0.02em',
        }}
      >
        {title}
      </h2>
      {children}
    </section>
  )
}
