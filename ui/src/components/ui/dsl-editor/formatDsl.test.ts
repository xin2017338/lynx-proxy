import { beforeAll, describe, expect, it } from 'vitest'

import { dslWasmAvailable, setupDslWasmForTests } from './dslTestWasm'
import {
  canFormatDsl,
  formatDsl,
  isDslFormatted,
} from './formatDsl'

describe.skipIf(!dslWasmAvailable)('formatDsl', () => {
  beforeAll(async () => {
    await setupDslWasmForTests()
  }, 120_000)

  it('formats AND with normalized spacing and uppercase operators', () => {
    expect(formatDsl('example.com   and   /api')).toBe('example.com AND /api')
  })

  it('formats top-level OR onto separate lines', () => {
    expect(formatDsl('example.com and /api OR https://example.com/health')).toBe(
      'example.com AND /api\nOR https://example.com/health',
    )
  })

  it('formats three top-level OR branches', () => {
    expect(formatDsl('example.com OR /api/ OR /health')).toBe(
      'example.com\nOR /api/\nOR /health',
    )
  })

  it('accepts bare protocols as request matchers', () => {
    expect(formatDsl('https:// or wss://')).toBe('https://\nOR wss://')
  })

  it('formats grouped OR inside parentheses across multiple lines', () => {
    expect(formatDsl('( example.com  or  /api/ )   and   not   https://example.com/health')).toBe(
      '(\n  example.com\n  OR /api/\n)\nAND NOT https://example.com/health',
    )
  })

  it('preserves leading full-line comments', () => {
    const input = '# match api traffic\nexample.com   and   /api\nOR   https://example.com/health'
    expect(formatDsl(input)).toBe(
      '# match api traffic\nexample.com AND /api\nOR https://example.com/health',
    )
  })

  it('preserves trailing same-line comments', () => {
    expect(formatDsl('example.com and /api # production')).toBe(
      'example.com AND /api # production',
    )
  })

  it('leaves comment-only documents unchanged', () => {
    expect(formatDsl('# notes only')).toBe('# notes only')
  })

  it('preserves CLI flag casing from the source slice', () => {
    expect(formatDsl('example.com -h x-token=b --header foo=bar   and   /api')).toBe(
      'example.com -h x-token=b --header foo=bar AND /api',
    )
  })

  it('formats long combined expressions', () => {
    expect(formatDsl('example.com AND /api/v1/events/track OR https://example.com:443/health')).toBe(
      'example.com AND /api/v1/events/track\nOR https://example.com:443/health',
    )
  })

  it('formats the Format story messy sample', () => {
    const input = `# filter: api + health exclusion
( example.com  or  /api/ )   and   not   https://example.com/health
OR   /public/**`
    expect(formatDsl(input)).toBe(
      `# filter: api + health exclusion
(
  example.com
  OR /api/
)
AND NOT https://example.com/health
OR /public/**`,
    )
  })

  it('returns null for invalid syntax', () => {
    expect(formatDsl('example.com AND (')).toBeNull()
  })

  it('returns null for empty content', () => {
    expect(formatDsl('   ')).toBeNull()
    expect(canFormatDsl('   ')).toBe(false)
  })

  it('detects already formatted documents', () => {
    const formatted = 'example.com AND /api\nOR https://example.com/health'
    expect(isDslFormatted(formatted)).toBe(true)
    expect(isDslFormatted('example.com and /api')).toBe(false)
  })
})
