import type { ComposeResponse } from '../types'
import type { WorkbenchLanguage } from '@/components/ui/content-workbench/utils'

export const RESPONSE_RENDER_LIMIT = 1024 * 1024
export const RESPONSE_PREVIEW_CHARS = 32 * 1024
export function responseMime(response?: ComposeResponse | null): string {
  return Object.entries(response?.headers ?? {}).find(([key]) => key.toLowerCase() === 'content-type')?.[1]?.split(';')[0]?.trim().toLowerCase() ?? ''
}
export function responseKind(response?: ComposeResponse | null): WorkbenchLanguage | 'image' | 'audio' | 'video' | 'binary' {
  const mime = responseMime(response)
  if (mime.startsWith('image/')) return 'image'
  if (mime.startsWith('audio/')) return 'audio'
  if (mime.startsWith('video/')) return 'video'
  if (response?.bodyEncoding === 'base64') return 'binary'
  if (mime.includes('json')) return 'json'
  if (mime.includes('javascript') || mime.includes('ecmascript')) return 'javascript'
  if (mime.includes('css')) return 'css'
  if (mime.includes('html')) return 'html'
  if (mime.includes('xml')) return 'xml'
  if (!mime || mime === 'text/plain') {
    const sample = response?.body.trimStart() ?? ''
    if (/^[\[{]/.test(sample) && sample.length <= RESPONSE_RENDER_LIMIT) {
      try { JSON.parse(sample); return 'json' } catch { /* Keep text fallback. */ }
    }
  }
  return 'plaintext'
}
export function responseTabs(response?: ComposeResponse | null): string[] {
  if (response?.bodyState || response?.errorMessage) return ['notice', 'headers']
  if (!response?.body) return ['headers']
  const kind = responseKind(response)
  if (['image', 'audio', 'video'].includes(kind)) return ['preview', 'raw', 'headers']
  if (kind === 'binary') return ['file', 'raw', 'headers']
  return kind === 'plaintext' || (response?.size ?? 0) > RESPONSE_RENDER_LIMIT || (response?.body.length ?? 0) > RESPONSE_RENDER_LIMIT
    ? ['raw', 'headers'] : ['formatted', 'raw', 'headers']
}
export function responseBytes(response: ComposeResponse): Uint8Array {
  if (response.bodyEncoding !== 'base64') return new TextEncoder().encode(response.body)
  const padding = response.body.endsWith('==') ? 2 : response.body.endsWith('=') ? 1 : 0
  const bytes = new Uint8Array(Math.floor(response.body.length * 3 / 4) - padding)
  let offset = 0
  for (let i = 0; i < response.body.length; i += 32768) {
    const chunk = atob(response.body.slice(i, i + 32768))
    for (let j = 0; j < chunk.length; j++) bytes[offset++] = chunk.charCodeAt(j)
  }
  return bytes
}
export function responseHex(response: ComposeResponse): string {
  const sample = response.bodyEncoding === 'base64'
    ? Uint8Array.from(atob(response.body.slice(0, 4096)), c => c.charCodeAt(0))
    : new TextEncoder().encode(response.body.slice(0, 1024))
  return Array.from({ length: Math.ceil(sample.length / 16) }, (_, row) => {
    const bytes = sample.slice(row * 16, row * 16 + 16)
    return `${(row * 16).toString(16).padStart(8, '0')}  ${Array.from(bytes, b => b.toString(16).padStart(2, '0')).join(' ').padEnd(47)}  ${Array.from(bytes, b => b >= 32 && b < 127 ? String.fromCharCode(b) : '.').join('')}`
  }).join('\n')
}
