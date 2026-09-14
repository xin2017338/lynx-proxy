import { normalizeComposeResponse } from './history-response'
import type { ApiRequestTab, ApiStudioSidebarView, ComposeResponse } from '../types'

export const API_STUDIO_TAB_SESSION_KEY = 'lynx.apiStudio.tabSession'
export const API_STUDIO_TAB_SESSION_VERSION = 1 as const

export type PersistedTabRef =
  | {
      kind: 'request'
      requestNodeId: string
      activeRequestTab?: ApiRequestTab
      lastHistoryEntryId?: string
      /** Cached response for instant restore before / after history list loads. */
      lastResponse?: ComposeResponse
    }
  | { kind: 'history'; entryId: string; activeRequestTab?: ApiRequestTab }

export type PersistedActiveTabRef =
  | { kind: 'request'; requestNodeId: string }
  | { kind: 'history'; entryId: string }

export interface ApiStudioTabSession {
  version: typeof API_STUDIO_TAB_SESSION_VERSION
  tabs: PersistedTabRef[]
  active?: PersistedActiveTabRef
  selectedId?: string
  sidebarView?: ApiStudioSidebarView
}

function isRequestTabRef(value: unknown): value is Extract<PersistedTabRef, { kind: 'request' }> {
  if (!value || typeof value !== 'object') return false
  const row = value as PersistedTabRef
  return row.kind === 'request' && typeof row.requestNodeId === 'string' && row.requestNodeId.length > 0
}

function isHistoryTabRef(value: unknown): value is Extract<PersistedTabRef, { kind: 'history' }> {
  if (!value || typeof value !== 'object') return false
  const row = value as PersistedTabRef
  return row.kind === 'history' && typeof row.entryId === 'string' && row.entryId.length > 0
}

function isActiveRef(value: unknown): value is PersistedActiveTabRef {
  if (!value || typeof value !== 'object') return false
  const row = value as PersistedActiveTabRef
  if (row.kind === 'request') {
    return typeof row.requestNodeId === 'string' && row.requestNodeId.length > 0
  }
  if (row.kind === 'history') {
    return typeof row.entryId === 'string' && row.entryId.length > 0
  }
  return false
}

const REQUEST_EDITOR_TABS = new Set<ApiRequestTab>([
  'params',
  'headers',
  'body',
  'cookies',
  'settings',
])

function sanitizeRequestTab(value: unknown): ApiRequestTab | undefined {
  return typeof value === 'string' && REQUEST_EDITOR_TABS.has(value as ApiRequestTab)
    ? value as ApiRequestTab
    : undefined
}

function sanitizeSidebarView(value: unknown): ApiStudioSidebarView | undefined {
  return value === 'collections' || value === 'history' ? value : undefined
}

function sanitizeLastResponse(value: unknown): ComposeResponse | undefined {
  const normalized = normalizeComposeResponse(value as ComposeResponse | null)
  if (!normalized || normalized.status <= 0) return undefined
  return normalized
}

export function parseApiStudioTabSession(raw: string | null): ApiStudioTabSession | null {
  if (!raw?.trim()) return null
  try {
    const parsed = JSON.parse(raw) as Partial<ApiStudioTabSession>
    if (parsed.version !== API_STUDIO_TAB_SESSION_VERSION) return null

    const tabs: PersistedTabRef[] = []
    if (Array.isArray(parsed.tabs)) {
      for (const item of parsed.tabs) {
        if (isRequestTabRef(item)) {
          const row = item as Extract<PersistedTabRef, { kind: 'request' }>
          tabs.push({
            kind: 'request',
            requestNodeId: row.requestNodeId,
            activeRequestTab: sanitizeRequestTab(row.activeRequestTab),
            lastHistoryEntryId:
              typeof row.lastHistoryEntryId === 'string' && row.lastHistoryEntryId.length > 0
                ? row.lastHistoryEntryId
                : undefined,
            lastResponse: sanitizeLastResponse(row.lastResponse),
          })
        }
        else if (isHistoryTabRef(item)) {
          tabs.push({
            kind: 'history',
            entryId: item.entryId,
            activeRequestTab: sanitizeRequestTab(item.activeRequestTab),
          })
        }
      }
    }

    const active = isActiveRef(parsed.active) ? parsed.active : undefined
    const selectedId = typeof parsed.selectedId === 'string' && parsed.selectedId.length > 0
      ? parsed.selectedId
      : undefined
    const sidebarView = sanitizeSidebarView(parsed.sidebarView)

    if (tabs.length === 0 && !active && !selectedId) {
      return null
    }

    return {
      version: API_STUDIO_TAB_SESSION_VERSION,
      tabs,
      active,
      selectedId,
      sidebarView,
    }
  }
  catch {
    return null
  }
}

export function readApiStudioTabSession(): ApiStudioTabSession | null {
  if (typeof window === 'undefined') return null
  return parseApiStudioTabSession(window.localStorage.getItem(API_STUDIO_TAB_SESSION_KEY))
}

export function writeApiStudioTabSession(session: ApiStudioTabSession): void {
  if (typeof window === 'undefined') return
  const compact = { ...session, tabs: session.tabs.map(tab => tab.kind === 'request' && tab.lastResponse && tab.lastResponse.body.length > 16 * 1024
    ? { ...tab, lastResponse: { ...tab.lastResponse, body: '', bodyEncoding: undefined, bodyState: 'omitted', bodyMessage: '标签缓存未保存较大的正文，请从历史记录恢复或重新发送请求。' } }
    : tab) }
  try { window.localStorage.setItem(API_STUDIO_TAB_SESSION_KEY, JSON.stringify(compact)) }
  catch {
    // Quota exhaustion must not interrupt an active request or destroy the previous session.
  }
}

export function clearApiStudioTabSession(): void {
  if (typeof window === 'undefined') return
  window.localStorage.removeItem(API_STUDIO_TAB_SESSION_KEY)
}

export function tabIdForHistoryEntry(entryId: string): string {
  return `tab-history-${entryId}`
}

export function activeTabRefFromTab(
  tab: { id: string; requestNodeId: string },
  historyEntryId?: string,
): PersistedActiveTabRef | undefined {
  if (tab.requestNodeId) {
    return { kind: 'request', requestNodeId: tab.requestNodeId }
  }
  if (historyEntryId) {
    return { kind: 'history', entryId: historyEntryId }
  }
  const match = tab.id.match(/^tab-history-(.+)$/)
  if (match?.[1]) {
    return { kind: 'history', entryId: match[1] }
  }
  return undefined
}

export function resolveActiveTabId(
  active: PersistedActiveTabRef | undefined,
  openTabs: Array<{ id: string; requestNodeId: string }>,
): string | undefined {
  if (!active) return openTabs[0]?.id
  if (active.kind === 'request') {
    return openTabs.find(tab => tab.requestNodeId === active.requestNodeId)?.id
  }
  const historyTabId = tabIdForHistoryEntry(active.entryId)
  return openTabs.find(tab => tab.id === historyTabId)?.id
}
