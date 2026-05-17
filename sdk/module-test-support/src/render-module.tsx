import { render, type RenderOptions, type RenderResult } from '@testing-library/react'
import type { ReactElement, ReactNode } from 'react'

import { PortakiProvider } from '@portaki/sdk'
import type { HostModuleContext, ModuleContext, PortakiModuleDefinition } from '@portaki/module-sdk'

import {
  createMockHostModuleContext,
  createMockModuleContext,
  createMockPortakiRuntimeValue,
  type MockModuleContextOverrides,
} from './fixtures.js'

export type RenderGuestModuleOptions = MockModuleContextOverrides & {
  renderOptions?: Omit<RenderOptions, 'wrapper'>
}

export function renderGuestModule(
  module: PortakiModuleDefinition,
  options: RenderGuestModuleOptions = {},
): RenderResult & { ctx: ModuleContext } {
  const ctx = createMockModuleContext(options)
  const ui = module.render(ctx) as ReactElement
  const runtime = createMockPortakiRuntimeValue({ moduleId: module.id, lang: ctx.lang })
  const { hmacKeyMaterialB64, ...portakiContext } = runtime
  const Wrapper = ({ children }: { children: ReactNode }) => (
    <PortakiProvider context={portakiContext} hmacKeyMaterialB64={hmacKeyMaterialB64}>
      {children}
    </PortakiProvider>
  )
  const result = render(ui, { wrapper: Wrapper, ...options.renderOptions })
  return { ...result, ctx }
}

export function renderHostModule(
  module: PortakiModuleDefinition,
  ctx: HostModuleContext = createMockHostModuleContext(),
  renderOptions?: Omit<RenderOptions, 'wrapper'>,
): RenderResult & { ctx: HostModuleContext } {
  if (!module.renderHost) {
    throw new Error(`module ${module.id} has no renderHost`)
  }
  const ui = module.renderHost(ctx) as ReactElement
  const result = render(ui, renderOptions)
  return { ...result, ctx }
}
