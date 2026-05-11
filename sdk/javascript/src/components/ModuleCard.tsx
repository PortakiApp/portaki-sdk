import type { ReactNode } from 'react'

export interface ModuleCardProps {
  children: ReactNode
  className?: string
}

export function ModuleCard({ children, className }: ModuleCardProps) {
  return (
    <div
      className={className}
      style={{
        borderRadius: '16px',
        border: '1px solid color-mix(in srgb, var(--color-secondary, #2b7fbf) 18%, transparent)',
        background: 'color-mix(in srgb, var(--color-surface, #fff) 92%, transparent)',
        boxShadow: 'var(--shadow-card, 0 8px 32px rgba(9, 9, 11, 0.08))',
        backdropFilter: 'blur(12px)',
        padding: '20px 22px',
      }}
    >
      {children}
    </div>
  )
}
