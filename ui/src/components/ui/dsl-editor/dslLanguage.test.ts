import { describe, expect, it } from 'vitest'

import { dslHighlightColors } from './dslLanguage'

describe('dslHighlightColors', () => {
  it('assigns a unique color to each URL segment and keyword', () => {
    const values = Object.values(dslHighlightColors)
    expect(new Set(values).size).toBe(values.length)
  })

  it('does not reuse keyword color for host', () => {
    expect(dslHighlightColors.host).not.toBe(dslHighlightColors.keyword)
  })

  it('uses a distinct color for fuzzy keyword and host', () => {
    expect(dslHighlightColors.fuzzyKeyword).not.toBe(dslHighlightColors.host)
  })

})
