import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import { ModuleCard } from './ModuleCard.js'

describe('ModuleCard', () => {
  it('whenChildrenProvided_thenRendersContent', () => {
    render(<ModuleCard>Hello module</ModuleCard>)
    expect(screen.getByText('Hello module')).toBeInTheDocument()
  })
})
