import { ref, watch } from 'vue'
import { defineStore } from 'pinia'
import { WsOp } from '@/lib/generated/ws/v1'
import type { GeneralSetting } from '@/lib/http/settings-types'
import { MAX_LOG_SIZE_MAX, MAX_LOG_SIZE_MIN } from '@/lib/http/settings-types'
import { useWsConnectionStore } from './ws-connection.store'

export const DEFAULT_MAX_LOG_SIZE = 5000
export const DEFAULT_HIDE_CONNECT_TUNNELS = true

export function clampMaxLogSize(value: number): number {
  if (!Number.isFinite(value)) {
    return DEFAULT_MAX_LOG_SIZE
  }

  return Math.min(MAX_LOG_SIZE_MAX, Math.max(MAX_LOG_SIZE_MIN, Math.round(value)))
}

export function resolveHideConnectTunnels(value: boolean | undefined): boolean {
  return value !== false
}

export const useGeneralSettingsStore = defineStore('generalSettings', () => {
  const wsConnectionStore = useWsConnectionStore()
  const maxLogSize = ref(DEFAULT_MAX_LOG_SIZE)
  const hideConnectTunnels = ref(DEFAULT_HIDE_CONNECT_TUNNELS)

  async function load() {
    if (!wsConnectionStore.isConnected) {
      return
    }

    try {
      const general = await wsConnectionStore.call<GeneralSetting>(WsOp.SettingsGeneralGet)
      maxLogSize.value = clampMaxLogSize(general.maxLogSize)
      hideConnectTunnels.value = resolveHideConnectTunnels(general.hideConnectTunnels)
    } catch {
      // Keep current value when settings cannot be fetched.
    }
  }

  function applyMaxLogSize(value: number) {
    maxLogSize.value = clampMaxLogSize(value)
  }

  function applyHideConnectTunnels(value: boolean) {
    hideConnectTunnels.value = value
  }

  watch(
    () => wsConnectionStore.isConnected,
    (connected) => {
      if (connected) {
        void load()
      }
    },
    { immediate: true },
  )

  return {
    maxLogSize,
    hideConnectTunnels,
    load,
    applyMaxLogSize,
    applyHideConnectTunnels,
  }
})
