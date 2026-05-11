'use client'

import type { ReactNode } from 'react'
import { useCallback, useState } from 'react'

export interface CopyButtonProps {
  text: string
  children?: ReactNode
  className?: string
}

export function CopyButton({ text, children, className }: CopyButtonProps) {
  const [done, setDone] = useState(false)

  const onClick = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(text)
      setDone(true)
      window.setTimeout(() => setDone(false), 2000)
    } catch {
      setDone(false)
    }
  }, [text])

  return (
    <button
      type="button"
      className={className}
      onClick={onClick}
      style={{
        fontSize: '13px',
        fontWeight: 600,
        padding: '6px 10px',
        borderRadius: '8px',
        border: '1px solid color-mix(in srgb, var(--color-muted, #71717a) 35%, transparent)',
        background: 'var(--color-surface, #fff)',
        cursor: 'pointer',
      }}
    >
      {done ? 'Copié ✓' : children ?? 'Copier'}
    </button>
  )
}
