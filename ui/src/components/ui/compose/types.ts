export type ComposeHttpMethod =
  | 'GET'
  | 'POST'
  | 'PUT'
  | 'PATCH'
  | 'DELETE'
  | 'HEAD'
  | 'OPTIONS'

export const COMPOSE_HTTP_METHODS: ComposeHttpMethod[] = [
  'GET',
  'POST',
  'PUT',
  'PATCH',
  'DELETE',
  'HEAD',
  'OPTIONS',
]

export interface KeyValueRow {
  key: string
  value: string
  enabled: boolean
}

export interface ComposeDraft {
  name: string
  method: ComposeHttpMethod
  url: string
  queryParams: KeyValueRow[]
  headers: KeyValueRow[]
  body: string
  timeout?: number
}

export interface ComposeResponse {
  status: number
  statusText: string
  headers: Record<string, string>
  body: string
  responseTime: number
  size: number
  bodyEncoding?: 'base64'
  bodyState?: 'blocked' | 'failed' | 'omitted'
  bodyMessage?: string
  sizeKnown?: boolean
  limitBytes?: number
  errorMessage?: string
}

export type ComposeRequestTab = 'params' | 'headers' | 'body'

export type ComposeLayout = 'vertical' | 'horizontal'
