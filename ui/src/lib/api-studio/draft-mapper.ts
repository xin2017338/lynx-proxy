import type { ApiRequestDraft } from '@/components/ui/api-studio/types'
import type { ComposeHttpMethod } from '@/components/ui/compose/types'
import { createEmptyApiDraft } from '@/components/ui/api-studio/lib/empty-api-draft'
import {
  composeRequestUrl,
  normalizeDraftUrlParams,
} from '@/components/ui/compose/lib/parse-url-params'

/** Draft JSON returned by `/api/api_studio/drafts/:id`. */
export interface ServerApiStudioDraft {
  id: string
  name: string
  method: ComposeHttpMethod
  url: string
  queryParams?: ApiRequestDraft['queryParams']
  headers?: ApiRequestDraft['headers']
  body?: string
  cookies?: ApiRequestDraft['cookies']
  settings?: ApiRequestDraft['settings']
  timeout?: number
  createdAt?: number
  updatedAt?: number
}

export function serverDraftToApiDraft(
  server: ServerApiStudioDraft,
  overrides: Partial<ApiRequestDraft> = {},
): ApiRequestDraft {
  return normalizeDraftUrlParams(
    createEmptyApiDraft({
      name: server.name,
      method: server.method,
      url: server.url,
      queryParams: server.queryParams ?? [],
      headers: server.headers ?? [],
      body: server.body ?? '',
      cookies: server.cookies ?? [],
      settings: server.settings,
      timeout: server.timeout,
      ...overrides,
    }),
  ) as ApiRequestDraft
}

export function apiDraftToComposePayload(draft: ApiRequestDraft) {
  const timeoutSecs = (draft.settings?.timeoutMs
      ? Math.max(1, Math.ceil(draft.settings.timeoutMs / 1000))
      : draft.timeout ?? 30)

  return {
    method: draft.method,
    url: composeRequestUrl(draft.url, draft.queryParams),
    queryParams: [],
    headers: draft.headers,
    body: draft.body,
    timeout: timeoutSecs,
  }
}
