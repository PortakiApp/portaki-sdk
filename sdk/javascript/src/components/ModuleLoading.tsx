export function ModuleLoading() {
  return (
    <div
      aria-busy
      aria-label="Chargement"
      style={{
        height: '120px',
        borderRadius: '12px',
        background:
          'linear-gradient(90deg, rgba(9,9,11,0.06) 25%, rgba(9,9,11,0.12) 37%, rgba(9,9,11,0.06) 63%)',
        backgroundSize: '400% 100%',
        animation: 'portaki-shimmer 1.2s ease-in-out infinite',
      }}
    >
      <style>{`@keyframes portaki-shimmer { 0% { background-position: 100% 0; } 100% { background-position: 0 0; } }`}</style>
    </div>
  )
}
