import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import { ModuleError } from './ModuleError.js'

describe('ModuleError', () => {
  it('whenCustomMessage_thenRendersTitleAndMessage', () => {
    render(<ModuleError title="Oups" message="Réessayez plus tard." />)
    expect(screen.getByText('Oups')).toBeInTheDocument()
    expect(screen.getByText(/Réessayez plus tard/)).toBeInTheDocument()
  })

  it('whenDefaults_thenRendersDefaultCopy', () => {
    render(<ModuleError />)
    expect(screen.getByText('Erreur')).toBeInTheDocument()
    expect(screen.getByText(/Impossible de charger/)).toBeInTheDocument()
  })
})
