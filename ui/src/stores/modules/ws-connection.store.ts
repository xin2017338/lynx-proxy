import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import {
  createDefaultWsClient,
  WsConnectionState,
  type WsClient,
  type WsConnectionState as WsConnectionStateType,
} from '@/lib/ws'
import type { WsEventFrame } from '@/lib/generated/ws/v1'

const MAX_EVENT_LOG = 200

export const useWsConnectionStore = defineStore('wsConnection', () => {
  const state = ref<WsConnectionStateType>(WsConnectionState.Disconnected)
  const lastError = ref<string | null>(null)
  const recentEvents = ref<Array<WsEventFrame<Record<string, unknown>>>>([])

  let wsClient: WsClient | null = null
  let detachStateChange: (() => void) | null = null
  let detachEvent: (() => void) | null = null

  const isConnected = computed(() => state.value === WsConnectionState.Connected)

  const bindClient = (client: WsClient) => {
    detachStateChange?.()
    detachEvent?.()

    wsClient = client

    detachStateChange = wsClient.onStateChange((nextState, reason) => {
      state.value = nextState
      if (reason) {
        lastError.value = reason
      }
    })

    detachEvent = wsClient.onEvent((frame) => {
      recentEvents.value.push(frame)
      if (recentEvents.value.length > MAX_EVENT_LOG) {
        recentEvents.value.shift()
      }
    })
  }

  const ensureClient = () => {
    if (!wsClient) {
      bindClient(createDefaultWsClient())
    }

    return wsClient!
  }

  const resetClient = () => {
    detachStateChange?.()
    detachEvent?.()
    detachStateChange = null
    detachEvent = null
    wsClient?.disconnect()
    wsClient = null
    state.value = WsConnectionState.Disconnected
  }

  const bootstrap = () => {
    ensureClient()
    connect().catch((error) => {
      lastError.value = String(error)
    })
  }

  const reconnect = async () => {
    resetClient()
    bootstrap()
    await connect()
  }

  const connect = async () => {
    await ensureClient().connect()
  }

  const disconnect = () => {
    wsClient?.disconnect()
  }

  const call = async <TResponse = unknown, TPayload = Record<string, unknown>>(
    op: string,
    payload?: TPayload,
    timeoutMs?: number,
  ) => {
    return ensureClient().call<TResponse, TPayload>(op, payload, timeoutMs)
  }

  const onEvent = (
    handler: (frame: WsEventFrame<Record<string, unknown>>) => void,
  ) => {
    return ensureClient().onEvent(handler)
  }

  const clearRecentEvents = () => {
    recentEvents.value = []
  }

  const dispose = () => {
    detachStateChange?.()
    detachEvent?.()
    detachStateChange = null
    detachEvent = null
    wsClient?.disconnect(1000, 'Store disposed')
    wsClient = null
  }

  return {
    state,
    isConnected,
    lastError,
    recentEvents,
    bootstrap,
    reconnect,
    connect,
    disconnect,
    call,
    onEvent,
    clearRecentEvents,
    dispose,
  }
})
