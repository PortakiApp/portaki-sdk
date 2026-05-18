import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import { ModuleLoading } from './ModuleLoading.js'

describe('ModuleLoading', () => {
  it('whenRendered_thenExposesBusyState', () => {
    render(<ModuleLoading />)
    expect(screen.getByLabelText('Chargement')).toHaveAttribute('aria-busy', 'true')
  })
})
