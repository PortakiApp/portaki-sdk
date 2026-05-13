export interface ModuleEmptyProps {
  message?: string
}

export function ModuleEmpty({ message = 'Aucun contenu pour le moment.' }: ModuleEmptyProps) {
  return (
    <p style={{ fontSize: '14px', color: 'var(--color-muted, #71717a)' }}>{message}</p>
  )
}
