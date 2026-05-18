import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'

import { CopyButton } from './CopyButton.js'

describe('CopyButton', () => {
  it('whenClicked_thenCopiesTextAndShowsConfirmation', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined)
    Object.assign(navigator, { clipboard: { writeText } })

    render(<CopyButton text="secret-code" />)
    fireEvent.click(screen.getByRole('button', { name: 'Copier' }))

    await waitFor(() => {
      expect(writeText).toHaveBeenCalledWith('secret-code')
    })
    expect(screen.getByRole('button', { name: 'Copié ✓' })).toBeInTheDocument()
  })
})
