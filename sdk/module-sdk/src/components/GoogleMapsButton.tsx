'use client'

export interface GoogleMapsButtonProps {
  lat: number
  lng: number
  label?: string
  className?: string
}

export function GoogleMapsButton({
  lat,
  lng,
  label = 'Google Maps',
  className,
}: GoogleMapsButtonProps) {
  const url = `https://www.google.com/maps/search/?api=1&query=${lat},${lng}`
  return (
    <a
      href={url}
      target="_blank"
      rel="noopener noreferrer"
      className={className}
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: '8px',
        padding: '10px 14px',
        borderRadius: '12px',
        fontWeight: 600,
        fontSize: '14px',
        background: 'color-mix(in srgb, var(--color-primary, #e8724a) 14%, white)',
        color: 'var(--color-text, #18181b)',
        textDecoration: 'none',
      }}
    >
      {label}
    </a>
  )
}
