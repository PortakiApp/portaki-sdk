export interface ModuleErrorProps {
  title?: string
  message?: string
}

export function ModuleError({
  title = 'Erreur',
  message = 'Impossible de charger ce module.',
}: ModuleErrorProps) {
  return (
    <p style={{ fontSize: '14px', color: 'var(--color-muted, #71717a)' }}>
      <strong style={{ color: 'var(--color-text, #18181b)' }}>{title}</strong>
      {' · '}
      {message}
    </p>
  )
}
