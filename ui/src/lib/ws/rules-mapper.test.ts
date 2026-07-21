import { describe, expect, it } from 'vitest'
import {
  cloneDraft,
  draftToRequestRule,
  extractProxyForwardUrl,
  requestRuleToDraft,
  requestRuleToListItem,
  sortRulesForDisplay,
} from './rules-mapper'
import type { RequestRuleDto } from './rules-types'

const sampleRule: RequestRuleDto = {
  id: 1,
  name: 'Test Rule',
  description: 'desc',
  enabled: true,
  priority: 50,
  capture: { id: 1, matchExpr: 'example.com AND /api' },
  handlers: [
    {
      id: 10,
      executionOrder: 100,
      enabled: true,
      handlerType: { type: 'block', statusCode: 403, reason: 'blocked' },
    },
  ],
}

const throttleRule: RequestRuleDto = {
  id: 2,
  name: 'Throttle Rule',
  description: 'desc',
  enabled: true,
  priority: 50,
  capture: { id: 2, matchExpr: '/api' },
  handlers: [
    {
      id: 20,
      executionOrder: 100,
      enabled: true,
      handlerType: {
        type: 'throttle',
        preset: 'fast3G',
        downloadKbps: undefined,
        uploadKbps: undefined,
        latencyMs: undefined,
      },
    },
  ],
}

describe('rules-mapper', () => {
  it('round-trips matchExpr and block handler', () => {
    const draft = requestRuleToDraft(sampleRule)
    expect(draft.matchDsl).toBe('example.com AND /api')
    expect(draft.actions[0]?.type).toBe('block')

    const back = draftToRequestRule(draft)
    expect(back.capture.matchExpr).toBe(sampleRule.capture.matchExpr)
    expect(back.handlers[0]?.handlerType.type).toBe('block')
  })

  it('round-trips throttle handler preset', () => {
    const draft = requestRuleToDraft(throttleRule)
    expect(draft.actions[0]?.type).toBe('throttle')
    if (draft.actions[0]?.type !== 'throttle') {
      throw new Error('expected throttle action')
    }
    expect(draft.actions[0].config.preset).toBe('Fast3G')

    const back = draftToRequestRule(draft)
    expect(back.handlers[0]?.handlerType.type).toBe('throttle')
    expect((back.handlers[0]?.handlerType as any).preset).toBe('fast3G')
  })

  it('builds list item with validation state', () => {
    const item = requestRuleToListItem(sampleRule)
    expect(item.id).toBe('1')
    expect(item.state).toBe('valid')
    expect(item.summary).toContain('example.com')
  })

  it('omits empty proxyForward fields when saving', () => {
    const draft = requestRuleToDraft({
      ...sampleRule,
      handlers: [
        {
          id: 11,
          executionOrder: 10,
          enabled: true,
          handlerType: {
            type: 'proxyForward',
            targetScheme: '',
            targetAuthority: '127.0.0.1:8000',
            targetPath: '  ',
          },
        },
      ],
    })
    expect(draft.actions[0]?.type).toBe('proxyForward')
    if (draft.actions[0]?.type !== 'proxyForward') {
      throw new Error('expected proxyForward action')
    }
    expect(draft.actions[0].config.targetScheme).toBe('')
    expect(draft.actions[0].config.targetPath).toBe('')

    const back = draftToRequestRule(draft)
    const handler = back.handlers[0]?.handlerType
    expect(handler?.type).toBe('proxyForward')
    if (handler?.type !== 'proxyForward') {
      throw new Error('expected proxyForward handler')
    }
    expect(handler.targetScheme).toBeUndefined()
    expect(handler.targetAuthority).toBe('127.0.0.1:8000')
    expect(handler.targetPath).toBeUndefined()
  })

  it('normalizes ws/wss scheme values when loading proxyForward', () => {
    const draft = requestRuleToDraft({
      ...sampleRule,
      handlers: [
        {
          id: 12,
          executionOrder: 10,
          enabled: true,
          handlerType: {
            type: 'proxyForward',
            targetScheme: 'wss',
            targetAuthority: '127.0.0.1:8000',
          },
        },
      ],
    })
    if (draft.actions[0]?.type !== 'proxyForward') {
      throw new Error('expected proxyForward action')
    }
    expect(draft.actions[0].config.targetScheme).toBe('https')

    const back = draftToRequestRule(draft)
    const handler = back.handlers[0]?.handlerType
    if (handler?.type !== 'proxyForward') {
      throw new Error('expected proxyForward handler')
    }
    expect(handler.targetScheme).toBe('https')
  })

  it('cloneDraft deep-copies without structuredClone', () => {
    const draft = requestRuleToDraft(sampleRule)
    const copy = cloneDraft(draft)
    expect(copy).toEqual(draft)
    expect(copy).not.toBe(draft)
    copy.name = 'mutated'
    expect(draft.name).toBe('Test Rule')
  })

  it('extracts proxy forward url from first enabled handler', () => {
    const url = extractProxyForwardUrl([
      {
        id: 1,
        executionOrder: 10,
        enabled: false,
        handlerType: {
          type: 'proxyForward',
          targetScheme: 'https',
          targetAuthority: 'disabled.example.com',
        },
      },
      {
        id: 2,
        executionOrder: 20,
        enabled: true,
        handlerType: {
          type: 'proxyForward',
          targetScheme: 'https',
          targetAuthority: 'staging.example.com',
          targetPath: '/api',
        },
      },
    ])
    expect(url).toBe('https/wss://staging.example.com/api')
  })

  it('builds list item with forward url and effective enabled', () => {
    const item = requestRuleToListItem({
      ...sampleRule,
      project: 'staging',
      handlers: [
        {
          id: 11,
          executionOrder: 10,
          enabled: true,
          handlerType: {
            type: 'proxyForward',
            targetScheme: 'https',
            targetAuthority: 'staging.example.com',
          },
        },
      ],
    }, {
      projectEnabledMap: new Map([['staging', false]]),
      projectNameMap: new Map([['staging', 'Staging']]),
    })

    expect(item.forwardUrl).toBe('https/wss://staging.example.com')
    expect(item.projectId).toBe('staging')
    expect(item.projectName).toBe('Staging')
    expect(item.effectiveEnabled).toBe(false)
  })

  it('passes through createdAt and updatedAt', () => {
    const item = requestRuleToListItem({
      ...sampleRule,
      createdAt: 1000,
      updatedAt: 2000,
    })

    expect(item.createdAt).toBe(1000)
    expect(item.updatedAt).toBe(2000)
  })

  it('sorts rules by updatedAt descending with missing timestamps last', () => {
    const sorted = sortRulesForDisplay([
      { id: '1', name: 'A', enabled: true, priority: 10, updatedAt: 1000 },
      { id: '2', name: 'B', enabled: true, priority: 20, updatedAt: 3000 },
      { id: '3', name: 'C', enabled: true, priority: 30 },
      { id: '4', name: 'D', enabled: true, priority: 40, updatedAt: 2000 },
    ], 'updatedAt')

    expect(sorted.map(rule => rule.id)).toEqual(['2', '4', '1', '3'])
  })

  it('sorts rules by createdAt descending with missing timestamps last', () => {
    const sorted = sortRulesForDisplay([
      { id: '1', name: 'A', enabled: true, priority: 10, createdAt: 5000 },
      { id: '2', name: 'B', enabled: true, priority: 20 },
      { id: '3', name: 'C', enabled: true, priority: 30, createdAt: 9000 },
      { id: '4', name: 'D', enabled: true, priority: 40, createdAt: 7000 },
    ], 'createdAt')

    expect(sorted.map(rule => rule.id)).toEqual(['3', '4', '1', '2'])
  })

  it('sorts rules by forward url with empty urls last', () => {
    const sorted = sortRulesForDisplay([
      { id: '1', name: 'A', enabled: true, priority: 10, forwardUrl: 'https://b.example.com' },
      { id: '2', name: 'B', enabled: true, priority: 20 },
      { id: '3', name: 'C', enabled: true, priority: 30, forwardUrl: 'https://a.example.com' },
      { id: '4', name: 'D', enabled: true, priority: 40, forwardUrl: 'https://a.example.com' },
    ], 'forwardUrl')

    expect(sorted.map(rule => rule.id)).toEqual(['4', '3', '1', '2'])
  })
})
