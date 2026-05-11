'use client'

export interface WazeButtonProps {
  lat: number
  lng: number
  label?: string
  className?: string
}

export function WazeButton({ lat, lng, label = 'Waze', className }: WazeButtonProps) {
  const url = `https://waze.com/ul?ll=${lat},${lng}&navigate=yes`
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
        background: 'color-mix(in srgb, #33ccff 18%, white)',
        color: '#0369a1',
        textDecoration: 'none',
      }}
    >
      {label}
    </a>
  )
}
