import { cleanup, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it } from 'vitest'

import { ExternalLink } from './ExternalLink.js'
import { GoogleMapsButton } from './GoogleMapsButton.js'
import { ModuleConfigAlert } from './ModuleConfigAlert.js'
import { ModuleSection } from './ModuleSection.js'
import { WazeButton } from './WazeButton.js'

describe('module UI components (smoke)', () => {
  afterEach(() => {
    cleanup()
  })

  it('whenExternalLink_thenOpensInNewTab', () => {
    render(<ExternalLink href="https://example.com">Site</ExternalLink>)
    const link = screen.getByRole('link', { name: /Site/ })
    expect(link).toHaveAttribute('href', 'https://example.com')
    expect(link).toHaveAttribute('target', '_blank')
  })

  it('whenModuleSection_thenRendersTitle', () => {
    render(<ModuleSection title="Infos">Body</ModuleSection>)
    expect(screen.getByRole('heading', { name: 'Infos' })).toBeInTheDocument()
    expect(screen.getByText('Body')).toBeInTheDocument()
  })

  it('whenGoogleMapsButton_thenBuildsMapsUrl', () => {
    render(<GoogleMapsButton lat={43.3} lng={5.4} label="Carte" />)
    const link = screen.getByRole('link', { name: /Carte/ })
    expect(link.getAttribute('href')).toContain('google.com/maps')
  })

  it('whenWazeButton_thenBuildsWazeUrl', () => {
    render(<WazeButton lat={43.3} lng={5.4} />)
    const link = screen.getByRole('link', { name: 'Waze' })
    expect(link.getAttribute('href')).toContain('waze.com')
  })

  it('whenModuleConfigAlert_thenRendersMessage', () => {
    render(
      <ModuleConfigAlert
        lang="fr"
        alert={{
          type: 'warning',
          message: { fr: 'Vérifiez la config.', en: 'Check config.' },
        }}
      />,
    )
    expect(screen.getByText(/Vérifiez la config/)).toBeInTheDocument()
  })
})
