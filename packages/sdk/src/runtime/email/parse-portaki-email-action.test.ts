import { describe, expect, it } from 'vitest'

import { parsePortakiEmailAction } from './parse-portaki-email-action'

describe('parsePortakiEmailAction', () => {
  it('parses open-module with explicit action', () => {
    expect(parsePortakiEmailAction('open-module:checklist:fill-form')).toEqual({
      kind: 'open-module',
      moduleId: 'checklist',
      actionId: 'fill-form',
    })
  })

  it('defaults actionId when omitted', () => {
    expect(parsePortakiEmailAction('open-module:rules')).toEqual({
      kind: 'open-module',
      moduleId: 'rules',
      actionId: 'default',
    })
  })

  it('returns null for invalid payloads', () => {
    expect(parsePortakiEmailAction('')).toBeNull()
    expect(parsePortakiEmailAction('scroll:checklist')).toBeNull()
    expect(parsePortakiEmailAction('open-module:')).toBeNull()
  })
})
