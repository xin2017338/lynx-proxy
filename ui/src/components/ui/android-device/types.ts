export type AdbSource = 'none' | 'path' | 'bundled'

export type InstallPhase = 'idle' | 'downloading' | 'extracting' | 'done' | 'failed'

export interface AdbStatus {
  ready: boolean
  adbPath?: string | null
  version?: string | null
  source: AdbSource
}

export interface InstallProgress {
  phase: InstallPhase
  percent: number
  message?: string | null
  error?: string | null
}

export interface AdbDevice {
  serial: string
  state: string
  model?: string | null
  product?: string | null
}

export interface DeviceRowState {
  /** Actual value read from Android's global http_proxy setting. */
  currentHttpProxy?: string | null
  proxyStateError?: string | null
}

export interface ProxyState {
  serial: string
  currentHttpProxy?: string | null
  lynxManaged: boolean
}

export interface EnableProxyPayload {
  serial: string
  host: string
  port?: number
}

export interface AndroidDevicePreview {
  adbStatus: AdbStatus
  installProgress?: InstallProgress
  devices: AdbDevice[]
  deviceStates: Record<string, DeviceRowState>
  baseAddresses: string[]
  localOnly: boolean
  port: number
  busy?: boolean
  error?: string | null
}
