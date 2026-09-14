import { ref } from 'vue'
import { defineStore } from 'pinia'
import { WsOp } from '@/lib/generated/ws/v1'
import type {
  ApiRequestDraft,
  CollectionNode,
  ComposeResponse,
} from '@/components/ui/api-studio/types'
import type { CollectionPersistence } from '@/components/ui/api-studio/collection-persistence'
import {
  createCollectionNode,
  deleteCollectionNode,
  fetchCollectionNodes,
  fetchDraft,
  moveCollectionNode,
  renameCollectionNode,
  saveDraft,
} from '@/lib/http/api-studio-api'
import { apiDraftToComposePayload, serverDraftToApiDraft } from '@/lib/api-studio/draft-mapper'
import { normalizeComposeResponse } from '@/components/ui/api-studio/lib/history-response'
import { isRequestNode } from '@/components/ui/api-studio/lib/is-request-node'
import type { CollectionNodeKind } from '@/components/ui/api-studio/types'
import { useWsConnectionStore } from './ws-connection.store'

function normalizeCollectionNode(node: CollectionNode & { nodeType?: string }): CollectionNode {
  const rawKind = String(node.kind ?? node.nodeType ?? '').toLowerCase()
  let kind = (['collection', 'folder', 'request'].includes(rawKind)
    ? rawKind
    : node.draftId
      ? 'request'
      : 'folder') as CollectionNodeKind
  if (node.draftId) {
    kind = 'request'
  }
  return {
    ...node,
    kind,
    parentId: node.parentId ?? null,
  }
}

const SAVE_DEBOUNCE_MS = 500
const saveTimers = new Map<string, ReturnType<typeof setTimeout>>()

export const useApiStudioStore = defineStore('apiStudio', () => {
  const nodes = ref<CollectionNode[]>([])
  const drafts = ref<Record<string, ApiRequestDraft>>({})
  const selectedId = ref<string | undefined>(undefined)
  const loading = ref(false)
  const error = ref<string | null>(null)

  const ws = useWsConnectionStore()

  async function loadDraftIntoMap(draftId: string): Promise<ApiRequestDraft> {
    const server = await fetchDraft(draftId)
    const draft = serverDraftToApiDraft(server)
    drafts.value[draftId] = draft
    return draft
  }

  async function hydrate() {
    loading.value = true
    error.value = null
    try {
      const { nodes: nextNodes } = await fetchCollectionNodes()
      const normalized = nextNodes.map(normalizeCollectionNode)
      nodes.value = normalized

      const draftIds = normalized
        .filter(n => isRequestNode(n) && n.draftId)
        .map(n => n.draftId!)
      const loaded = await Promise.all(
        draftIds.map(async (id) => {
          try {
            const server = await fetchDraft(id)
            return [id, serverDraftToApiDraft(server)] as const
          }
          catch {
            return null
          }
        }),
      )
      const nextDrafts: Record<string, ApiRequestDraft> = {}
      for (const item of loaded) {
        if (item) nextDrafts[item[0]] = item[1]
      }
      drafts.value = nextDrafts
    }
    catch (err) {
      error.value = String(err)
    }
    finally {
      loading.value = false
    }
  }

  function cancelScheduledSave(draftId: string) {
    const existing = saveTimers.get(draftId)
    if (existing) {
      clearTimeout(existing)
      saveTimers.delete(draftId)
    }
  }

  function scheduleSaveDraft(draftId: string, draft: ApiRequestDraft) {
    cancelScheduledSave(draftId)
    saveTimers.set(
      draftId,
      setTimeout(() => {
        saveTimers.delete(draftId)
        void saveDraft(draftId, draft).catch((err) => {
          error.value = String(err)
        })
      }, SAVE_DEBOUNCE_MS),
    )
  }

  async function persistDraft(draftId: string, draft: ApiRequestDraft) {
    cancelScheduledSave(draftId)
    drafts.value[draftId] = draft
    await saveDraft(draftId, draft)
  }

  function syncDraftsFromShell(next: Record<string, ApiRequestDraft>) {
    for (const [draftId, draft] of Object.entries(next)) {
      drafts.value[draftId] = draft
      scheduleSaveDraft(draftId, draft)
    }
  }

  const collectionPersistence: CollectionPersistence = {
    async createNode(body) {
      const node = normalizeCollectionNode(await createCollectionNode(body))
      nodes.value = [...nodes.value, node]
      if (node.draftId) {
        await loadDraftIntoMap(node.draftId)
      }
      return node
    },
    async renameNode(id, name) {
      const node = normalizeCollectionNode(await renameCollectionNode(id, { name }))
      nodes.value = nodes.value.map(n => (n.id === id ? node : n))
      return node
    },
    async moveNode(id, body) {
      const node = normalizeCollectionNode(await moveCollectionNode(id, body))
      nodes.value = nodes.value.map(n => (n.id === id ? node : n))
      return node
    },
    async deleteNode(id) {
      await deleteCollectionNode(id)
      const remove = new Set<string>([id])
      const queue = [id]
      while (queue.length) {
        const current = queue.pop()!
        for (const n of nodes.value) {
          if (n.parentId === current && !remove.has(n.id)) {
            remove.add(n.id)
            queue.push(n.id)
          }
        }
      }
      for (const n of nodes.value) {
        if (n.kind === 'request' && n.draftId && remove.has(n.id)) {
          cancelScheduledSave(n.draftId)
          delete drafts.value[n.draftId]
        }
      }
      nodes.value = nodes.value.filter(n => !remove.has(n.id))
    },
    loadDraft: loadDraftIntoMap,
    async saveDraft(draftId, draft) {
      drafts.value[draftId] = draft
      await saveDraft(draftId, draft)
    },
  }

  async function sendRequest(draft: ApiRequestDraft): Promise<ComposeResponse | null> {
    if (!draft.url?.trim()) return null

    const payload = apiDraftToComposePayload(draft)
    const raw = await ws.call<ComposeResponse>(WsOp.ComposeRequestSend, payload, (payload.timeout + 10) * 1000)
    return normalizeComposeResponse(raw)
  }

  return {
    nodes,
    drafts,
    selectedId,
    loading,
    error,
    hydrate,
    syncDraftsFromShell,
    persistDraft,
    collectionPersistence,
    sendRequest,
  }
})
