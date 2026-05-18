import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import { ModuleEmpty } from './ModuleEmpty.js'

describe('ModuleEmpty', () => {
  it('whenMessageProvided_thenRendersMessage', () => {
    render(<ModuleEmpty message="Aucune donnée pour ce séjour." />)
    expect(screen.getByText('Aucune donnée pour ce séjour.')).toBeInTheDocument()
  })
})
