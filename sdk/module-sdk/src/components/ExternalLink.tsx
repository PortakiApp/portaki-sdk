import type { ReactNode } from 'react'

export interface ExternalLinkProps {
  href: string
  children: ReactNode
  className?: string
}

export function ExternalLink({ href, children, className }: ExternalLinkProps) {
  return (
    <a
      href={href}
      target="_blank"
      rel="noopener noreferrer"
      className={className}
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: '6px',
        fontSize: '14px',
        fontWeight: 600,
        color: 'var(--color-secondary, #2b7fbf)',
      }}
    >
      <span aria-hidden style={{ fontSize: '12px' }}>
        ↗
      </span>
      {children}
    </a>
  )
}
