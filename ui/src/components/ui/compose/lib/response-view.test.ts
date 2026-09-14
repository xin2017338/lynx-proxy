import { describe, expect, it } from 'vitest'
import { responseBytes, responseKind, responseTabs } from './response-view'
import type { ComposeResponse } from '../types'
import { snapshotResponseForHistory } from '@/components/ui/api-studio/lib/history-response'
const response = (mime: string, body = '{}', extra: Partial<ComposeResponse> = {}): ComposeResponse => ({ status: 200, statusText: 'OK', headers: { 'Content-Type': mime }, body, responseTime: 1, size: body.length, ...extra })
describe('response presentation', () => {
  it.each([['application/problem+json', 'json'], ['text/javascript', 'javascript'], ['text/css', 'css'], ['text/html', 'html'], ['application/xml', 'xml'], ['image/png', 'image'], ['audio/wav', 'audio'], ['video/mp4', 'video']])('recognizes %s', (mime, kind) => { expect(responseKind(response(mime))).toBe(kind) })
  it('selects preview for media, headers for empty and notice for blocked', () => {
    expect(responseTabs(response('image/png'))[0]).toBe('preview')
    expect(responseTabs(response('', ''))).toEqual(['headers'])
    expect(responseTabs(response('text/plain', '', { bodyState: 'blocked' }))).toEqual(['notice', 'headers'])
  })
  it('does not auto-format large or invalid unknown JSON', () => {
    expect(responseTabs(response('application/json', '{}', { size: 8 * 1024 * 1024 }))).toEqual(['raw', 'headers'])
    expect(responseKind(response('', '{broken'))).toBe('plaintext')
    expect(responseKind(response('', '{"a":1}'))).toBe('json')
  })
  it('round trips binary data and its history metadata', () => {
    const source = response('image/png', 'AP+A', { bodyEncoding: 'base64' })
    expect(Array.from(responseBytes(source))).toEqual([0, 255, 128])
    expect(snapshotResponseForHistory(source)?.bodyEncoding).toBe('base64')
  })
  it('marks oversized history bodies as omitted and keeps headers', () => {
    const snap = snapshotResponseForHistory(response('text/plain', 'a'.repeat(300000)))!
    expect(snap.body).toBe('')
    expect(snap.bodyState).toBe('omitted')
    expect(snap.headers['Content-Type']).toBe('text/plain')
  })
})
