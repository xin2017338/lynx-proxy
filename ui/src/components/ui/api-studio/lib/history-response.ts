import { cloneDraft } from './clone-draft'
import type { ApiRequestDraft, ApiRequestHistoryEntry, ComposeResponse } from '../types'

/** Normalize WS/HTTP compose payloads so history always gets a string body. */
export function normalizeComposeResponse(
  response: ComposeResponse | null | undefined,
): ComposeResponse | null {
  if (!response) return null
  const rawBody = (response as { body?: unknown }).body
  const body = typeof rawBody === 'string'
    ? rawBody
    : rawBody != null
      ? String(rawBody)
      : ''
  return {
    status: response.status ?? 0,
    statusText: response.statusText ?? '',
    headers: { ...(response.headers ?? {}) },
    body,
    responseTime: response.responseTime ?? 0,
    size: response.size ?? body.length,
    errorMessage: response.errorMessage,
    bodyEncoding: response.bodyEncoding,
    bodyState: response.bodyState,
    bodyMessage: response.bodyMessage,
    sizeKnown: response.sizeKnown,
    limitBytes: response.limitBytes,
  }
}

/** Snapshot request draft for history persistence (includes request body). */
export function snapshotDraftForHistory(draft: ApiRequestDraft): ApiRequestDraft {
  const cloned = cloneDraft(draft)
  return {
    ...cloned,
    body: cloned.body ?? '',
  }
}

/** Build a displayable response from a history row (new or legacy entries). */
export function responseFromHistoryEntry(
  entry: ApiRequestHistoryEntry,
): ComposeResponse | null {
  if (entry.response) {
    return normalizeComposeResponse(entry.response)
  }
  if (entry.status == null) {
    return null
  }
  return {
    status: entry.status,
    statusText: entry.statusText ?? '',
    headers: {},
    body: '',
    responseTime: entry.durationMs ?? 0,
    size: 0,
  }
}

/** Ensure list/preview entries expose a normalized draft + response for the workbench. */
export function normalizeHistoryEntry(
  entry: ApiRequestHistoryEntry,
): ApiRequestHistoryEntry {
  const draft = snapshotDraftForHistory(entry.draft)
  const persisted = entry.response
    ? normalizeComposeResponse(entry.response)
    : null
  const response = persisted?.body?.trim()
    ? persisted
    : responseFromHistoryEntry({ ...entry, draft })
  return {
    ...entry,
    draft,
    response: response ?? undefined,
  }
}

/** Merge API row with the snapshot we just sent (server row may omit large bodies). */
export function mergeHistoryEntryWithSnapshot(
  entry: ApiRequestHistoryEntry,
  draft: ApiRequestDraft,
  snapshot: ComposeResponse | undefined,
): ApiRequestHistoryEntry {
  const normalizedDraft = snapshotDraftForHistory(
    entry.draft?.body?.trim() ? entry.draft : draft,
  )
  const serverResponse = entry.response
    ? normalizeComposeResponse(entry.response)
    : null
  const response = serverResponse?.body?.trim()
    ? serverResponse
    : snapshot ?? serverResponse ?? responseFromHistoryEntry(entry) ?? undefined
  return {
    ...entry,
    draft: normalizedDraft,
    response,
  }
}

/** Most recent history row linked to this open tab. */
export function findLatestHistoryEntryForTab(
  tab: { id: string; requestNodeId?: string; lastHistoryEntryId?: string },
  entries: ApiRequestHistoryEntry[],
): ApiRequestHistoryEntry | undefined {
  if (tab.lastHistoryEntryId) {
    const linked = entries.find(entry => entry.id === tab.lastHistoryEntryId)
    if (linked) return normalizeHistoryEntry(linked)
  }
  const matches = entries.filter((entry) => {
    if (tab.requestNodeId && entry.requestNodeId === tab.requestNodeId) {
      return true
    }
    return entry.tabId === tab.id
  })
  if (matches.length === 0) return undefined
  return normalizeHistoryEntry(
    matches.reduce((latest, entry) => (entry.sentAt > latest.sentAt ? entry : latest)),
  )
}

export function snapshotResponseForHistory(
  response: ComposeResponse | null | undefined,
): ComposeResponse | undefined {
  const normalized = normalizeComposeResponse(response)
  if (!normalized) return undefined
  if (normalized.body.length > 256 * 1024) {
    return { ...normalized, body: '', bodyEncoding: undefined, bodyState: 'omitted', bodyMessage: '此历史记录未保存较大的响应正文，请重新发送请求查看。' }
  }
  return normalized
}
