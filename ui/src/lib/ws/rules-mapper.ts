import type { RuleDraft, RuleActionDraft, RuleHeaderPair, RuleWorkbenchRuleItem } from '@/components/ui/rule-workbench'
import { createAction, createRuleDraft } from '@/components/ui/rule-workbench'
import { proxyForwardSchemeFromDto, proxyForwardSchemeSummaryLabel } from '@/components/ui/rule-workbench/proxy-forward-scheme'
import { getRuleValidationErrors } from '@/components/ui/rule-workbench/match-validation'
import type { ActionAssetTemplate } from '@/components/ui/rules-drawer/types'
import type { HandlerRuleDto, HandlerRuleTypeDto, RequestRuleDto } from './rules-types'

export interface RequestRuleToListItemOptions {
  projectEnabledMap?: Map<string, boolean>
  projectNameMap?: Map<string, string>
}

function isProjectEnabled(
  projectId: string,
  projectEnabledMap?: Map<string, boolean>,
): boolean {
  if (!projectEnabledMap) return true
  return projectEnabledMap.get(projectId) ?? true
}

/** First enabled proxyForward handler URL summary, or undefined. */
export function extractProxyForwardUrl(handlers: HandlerRuleDto[]): string | undefined {
  for (const handler of handlers) {
    if (!handler.enabled) continue
    const handlerType = handler.handlerType as HandlerRuleTypeDto
    if (handlerType.type !== 'proxyForward') continue
    const scheme = proxyForwardSchemeSummaryLabel(handlerType.targetScheme ?? '')
    const authority = (handlerType.targetAuthority ?? '').trim() || '<authority>'
    const path = (handlerType.targetPath ?? '').trim() || ''
    return `${scheme}://${authority}${path}`
  }
  return undefined
}

export function requestRuleToListItem(
  rule: RequestRuleDto,
  options?: RequestRuleToListItemOptions,
): RuleWorkbenchRuleItem {
  const draft = requestRuleToDraft(rule)
  const errors = getRuleValidationErrors(draft)
  const summary = rule.capture.matchExpr.length > 80
    ? `${rule.capture.matchExpr.slice(0, 77)}...`
    : rule.capture.matchExpr
  const projectId = rule.project ?? 'default'
  const projectEnabled = isProjectEnabled(projectId, options?.projectEnabledMap)
  const forwardUrl = extractProxyForwardUrl(rule.handlers)

  return {
    id: ruleIdToString(rule.id),
    name: rule.name,
    enabled: rule.enabled,
    effectiveEnabled: projectEnabled && rule.enabled,
    priority: rule.priority,
    summary,
    forwardUrl,
    projectId,
    projectName: options?.projectNameMap?.get(projectId),
    createdAt: rule.createdAt ?? null,
    updatedAt: rule.updatedAt ?? null,
    state: errors.length > 0 ? 'invalid' : 'valid',
  }
}

function normalizeThrottlePreset(raw?: string): 'Fast3G' | 'Slow3G' | 'Offline' | 'Custom' {
  const value = (raw ?? '').trim()
  if (!value) return 'Slow3G'
  const lowered = value.toLowerCase()
  if (lowered === 'fast3g') return 'Fast3G'
  if (lowered === 'slow3g') return 'Slow3G'
  if (lowered === 'offline') return 'Offline'
  if (lowered === 'custom') return 'Custom'
  return 'Slow3G'
}

function throttlePresetToDto(preset: 'Fast3G' | 'Slow3G' | 'Offline' | 'Custom'): string {
  if (preset === 'Fast3G') return 'fast3G'
  if (preset === 'Slow3G') return 'slow3G'
  if (preset === 'Offline') return 'offline'
  return 'custom'
}

function headersFromRecord(record?: Record<string, string>): RuleHeaderPair[] {
  if (!record) return []
  return Object.entries(record).map(([key, value]) => ({ key, value }))
}

function headersToRecord(headers: RuleHeaderPair[]): Record<string, string> | undefined {
  const pairs = headers.filter(h => h.key.trim())
  if (pairs.length === 0) return undefined
  return Object.fromEntries(pairs.map(h => [h.key, h.value]))
}

function optionalProxyForwardField(value: string): string | undefined {
  const trimmed = value.trim()
  return trimmed || undefined
}

function proxyForwardFieldFromDto(value?: string): string {
  return value?.trim() ?? ''
}

function handlerTypeToAction(handler: HandlerRuleDto, index: number): RuleActionDraft | null {
  const t = handler.handlerType as HandlerRuleTypeDto
  const base = {
    id: handler.id != null ? `act-${handler.id}` : `act-${index}`,
    enabled: handler.enabled,
    order: handler.executionOrder,
  }

  switch (t.type) {
    case 'block':
      return createAction({
        ...base,
        type: 'block',
        config: {
          statusCode: t.statusCode ?? 403,
          reason: t.reason ?? '',
        },
      })
    case 'delay':
      return createAction({
        ...base,
        type: 'delay',
        config: {
          delayMs: t.delayMs ?? 1000,
          varianceMs: t.varianceMs,
          delayType: t.delayType ?? 'beforeRequest',
        },
      })
    case 'proxyForward':
      return createAction({
        ...base,
        type: 'proxyForward',
        config: {
          targetScheme: proxyForwardSchemeFromDto(t.targetScheme),
          targetAuthority: proxyForwardFieldFromDto(t.targetAuthority),
          targetPath: proxyForwardFieldFromDto(t.targetPath),
        },
      })
    case 'modifyRequest':
      return createAction({
        ...base,
        type: 'modifyRequest',
        config: {
          modifyHeaders: headersFromRecord(t.modifyHeaders),
          modifyMethod: t.modifyMethod ?? '',
          modifyUrl: t.modifyUrl ?? '',
          modifyBody: t.modifyBody ?? '',
        },
      })
    case 'modifyResponse':
      return createAction({
        ...base,
        type: 'modifyResponse',
        config: {
          modifyHeaders: headersFromRecord(t.modifyHeaders),
          modifyStatusCode: t.modifyStatusCode,
          modifyBody: t.modifyBody ?? '',
        },
      })
    case 'localFile':
      return createAction({
        ...base,
        type: 'localFile',
        config: {
          filePath: t.filePath ?? '',
          contentType: t.contentType ?? '',
          statusCode: t.statusCode,
        },
      })
    case 'htmlScriptInjector': {
      const pos = t.injectionPosition ?? 'body-end'
      const injectionPosition = (pos === 'head' || pos === 'body-start' || pos === 'body-end')
        ? pos
        : 'body-end'
      return createAction({
        ...base,
        type: 'htmlScriptInjector',
        config: {
          content: t.content ?? '',
          injectionPosition,
        },
      })
    }
    case 'throttle':
      return createAction({
        ...base,
        type: 'throttle',
        config: {
          preset: normalizeThrottlePreset(t.preset),
          downloadKbps: t.downloadKbps,
          uploadKbps: t.uploadKbps,
          latencyMs: t.latencyMs,
        },
      })
    default:
      return null
  }
}

function actionToHandlerType(action: RuleActionDraft): HandlerRuleTypeDto {
  switch (action.type) {
    case 'block':
      return {
        type: 'block',
        statusCode: action.config.statusCode,
        reason: action.config.reason || undefined,
      }
    case 'delay':
      return {
        type: 'delay',
        delayMs: action.config.delayMs,
        varianceMs: action.config.varianceMs,
        delayType: action.config.delayType,
      }
    case 'proxyForward':
      return {
        type: 'proxyForward',
        targetScheme: optionalProxyForwardField(action.config.targetScheme),
        targetAuthority: optionalProxyForwardField(action.config.targetAuthority),
        targetPath: optionalProxyForwardField(action.config.targetPath),
      }
    case 'modifyRequest':
      return {
        type: 'modifyRequest',
        modifyHeaders: headersToRecord(action.config.modifyHeaders),
        modifyMethod: action.config.modifyMethod || undefined,
        modifyUrl: action.config.modifyUrl || undefined,
        modifyBody: action.config.modifyBody || undefined,
      }
    case 'modifyResponse':
      return {
        type: 'modifyResponse',
        modifyHeaders: headersToRecord(action.config.modifyHeaders),
        modifyStatusCode: action.config.modifyStatusCode,
        modifyBody: action.config.modifyBody || undefined,
      }
    case 'localFile':
      return {
        type: 'localFile',
        filePath: action.config.filePath,
        contentType: action.config.contentType || undefined,
        statusCode: action.config.statusCode,
      }
    case 'htmlScriptInjector':
      return {
        type: 'htmlScriptInjector',
        content: action.config.content,
        injectionPosition: action.config.injectionPosition,
      }
    case 'throttle':
      return {
        type: 'throttle',
        preset: throttlePresetToDto(action.config.preset),
        downloadKbps: action.config.preset === 'Custom' ? action.config.downloadKbps : undefined,
        uploadKbps: action.config.preset === 'Custom' ? action.config.uploadKbps : undefined,
        latencyMs: action.config.preset === 'Custom' ? action.config.latencyMs : undefined,
      }
  }
}

export function ruleIdToString(id?: number | null): string {
  if (id == null) return ''
  return String(id)
}

export function parseRuleId(id: string): number | null {
  if (!id || id.startsWith('draft-')) return null
  const n = Number.parseInt(id, 10)
  return Number.isFinite(n) ? n : null
}

export function requestRuleToDraft(rule: RequestRuleDto): RuleDraft {
  const actions = rule.handlers
    .map((h, i) => handlerTypeToAction(h, i))
    .filter((a): a is RuleActionDraft => a != null)
    .sort((a, b) => a.order - b.order)

  return createRuleDraft({
    id: ruleIdToString(rule.id),
    project: rule.project ?? 'default',
    name: rule.name,
    description: rule.description ?? '',
    enabled: rule.enabled,
    priority: rule.priority,
    matchDsl: rule.capture.matchExpr,
    actions: actions.length > 0 ? actions : undefined,
  })
}

export function draftToRequestRule(draft: RuleDraft, fallbackProject = 'default'): RequestRuleDto {
  const numericId = parseRuleId(draft.id)
  const handlers: HandlerRuleDto[] = [...draft.actions]
    .sort((a, b) => a.order - b.order)
    .map((action, index) => {
      const parsedId = action.id.match(/^act-(\d+)$/)
      return {
        id: parsedId ? Number.parseInt(parsedId[1]!, 10) : null,
        handlerType: actionToHandlerType(action),
        executionOrder: action.order || (index + 1) * 10,
        enabled: action.enabled,
      }
    })

  return {
    id: numericId,
    project: draft.project ?? fallbackProject,
    name: draft.name,
    description: draft.description || null,
    enabled: draft.enabled,
    priority: draft.priority,
    capture: {
      id: numericId,
      matchExpr: draft.matchDsl,
    },
    handlers,
  }
}


export function templateToAsset(template: HandlerRuleDto, index: number): ActionAssetTemplate | null {
  const action = handlerTypeToAction(template, index)
  if (!action) return null
  return {
    id: `tpl-${template.id ?? index}`,
    name: `模板 #${template.id ?? index}`,
    category: '模板',
    type: action.type,
    seedConfig: action.config,
  }
}

/** Plain JSON clone — works on Vue reactive proxies (structuredClone cannot). */
export function cloneDraft(draft: RuleDraft): RuleDraft {
  return JSON.parse(JSON.stringify(draft)) as RuleDraft
}

export type RuleListSortMode = 'updatedAt' | 'createdAt' | 'priority' | 'forwardUrl'

function compareTimestampDesc(
  left?: number | null,
  right?: number | null,
): number {
  const leftValue = left ?? null
  const rightValue = right ?? null
  if (leftValue == null && rightValue == null) return 0
  if (leftValue == null) return 1
  if (rightValue == null) return -1
  return rightValue - leftValue
}

function secondaryRuleSort(
  left: RuleWorkbenchRuleItem,
  right: RuleWorkbenchRuleItem,
): number {
  return right.priority - left.priority || left.name.localeCompare(right.name)
}

export function sortRulesForDisplay(
  rules: RuleWorkbenchRuleItem[],
  sortMode: RuleListSortMode,
): RuleWorkbenchRuleItem[] {
  if (sortMode === 'priority') {
    return [...rules]
  }

  return [...rules].sort((left, right) => {
    if (sortMode === 'updatedAt') {
      const timestampCompare = compareTimestampDesc(left.updatedAt, right.updatedAt)
      if (timestampCompare !== 0) return timestampCompare
      return secondaryRuleSort(left, right)
    }

    if (sortMode === 'createdAt') {
      const timestampCompare = compareTimestampDesc(left.createdAt, right.createdAt)
      if (timestampCompare !== 0) return timestampCompare
      return secondaryRuleSort(left, right)
    }

    const leftUrl = left.forwardUrl
    const rightUrl = right.forwardUrl
    if (!leftUrl && !rightUrl) {
      return secondaryRuleSort(left, right)
    }
    if (!leftUrl) return 1
    if (!rightUrl) return -1
    const urlCompare = leftUrl.localeCompare(rightUrl)
    if (urlCompare !== 0) return urlCompare
    return secondaryRuleSort(left, right)
  })
}
