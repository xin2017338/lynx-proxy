import type { AdbController } from './adb-controller'
import type {
  AdbDevice,
  AdbStatus,
  EnableProxyPayload,
  InstallProgress,
  ProxyState,
} from './types'

export interface MockAdbState {
  status: AdbStatus
  progress: InstallProgress
  devices: AdbDevice[]
  proxyBySerial: Record<string, ProxyState>
  baseAddresses: string[]
  localOnly: boolean
  port: number
}

export function createMockAdbController(state: () => MockAdbState): AdbController {
  return {
    async getStatus() {
      return state().status
    },
    async install() {
      const s = state()
      s.progress = { phase: 'downloading', percent: 10, message: 'Downloading…' }
      await delay(400)
      s.progress = { phase: 'extracting', percent: 90, message: 'Extracting…' }
      await delay(400)
      s.status = {
        ready: true,
        adbPath: '/mock/platform-tools/adb',
        version: 'Android Debug Bridge version 1.0.41',
        source: 'bundled',
      }
      s.progress = { phase: 'done', percent: 100, message: 'Ready' }
      return s.status
    },
    async getInstallProgress() {
      return state().progress
    },
    async listDevices() {
      return [...state().devices]
    },
    async getProxyState(serial) {
      const existing = state().proxyBySerial[serial]
      if (existing) {
        return existing
      }
      return {
        serial,
        currentHttpProxy: null,
        lynxManaged: false,
      }
    },
    async enableProxy(payload: EnableProxyPayload) {
      const s = state()
      const host = `${payload.host}:${payload.port ?? s.port}`
      const next: ProxyState = {
        serial: payload.serial,
        currentHttpProxy: host,
        lynxManaged: true,
      }
      s.proxyBySerial[payload.serial] = next
      return next
    },
    async disableProxy(serial) {
      const s = state()
      const next: ProxyState = {
        serial,
        currentHttpProxy: null,
        lynxManaged: false,
      }
      s.proxyBySerial[serial] = next
      return next
    },
    async getBaseAddresses() {
      return [...state().baseAddresses]
    },
    async getLocalOnly() {
      return state().localOnly
    },
  }
}

function delay(ms: number) {
  return new Promise<void>(resolve => {
    window.setTimeout(resolve, ms)
  })
}
