import type { AndroidDevicePreview, DeviceRowState } from './types'

export const FIXTURE_DEVICES = [
  {
    serial: 'emulator-5554',
    state: 'device',
    model: 'sdk_gphone64_x86_64',
    product: 'sdk_gphone64_x86_64',
  },
  {
    serial: 'R58M90ABCDE',
    state: 'unauthorized',
    model: 'SM-G991B',
    product: 'o1sxxx',
  },
] as const

export const FIXTURE_BASE_ADDRESSES = ['proxy.lynx.example:7788', 'backup.lynx.example:7788']

function states(entries: Record<string, DeviceRowState>): Record<string, DeviceRowState> {
  return entries
}

export const PREVIEW_READY_NO_DEVICES: AndroidDevicePreview = {
  adbStatus: { ready: true, source: 'bundled' },
  devices: [],
  deviceStates: {},
  baseAddresses: FIXTURE_BASE_ADDRESSES,
  localOnly: false,
  port: 7788,
}

export const PREVIEW_DEVICES_LISTED: AndroidDevicePreview = {
  ...PREVIEW_READY_NO_DEVICES,
  devices: [...FIXTURE_DEVICES],
  deviceStates: states({
    'emulator-5554': { currentHttpProxy: null },
    'R58M90ABCDE': { currentHttpProxy: null },
  }),
}

export const PREVIEW_ADB_NOT_INSTALLED: AndroidDevicePreview = {
  adbStatus: { ready: false, source: 'none' },
  installProgress: { phase: 'idle', percent: 0 },
  devices: [],
  deviceStates: {},
  baseAddresses: FIXTURE_BASE_ADDRESSES,
  localOnly: false,
  port: 7788,
}

export const PREVIEW_ADB_DOWNLOADING: AndroidDevicePreview = {
  ...PREVIEW_ADB_NOT_INSTALLED,
  installProgress: {
    phase: 'downloading',
    percent: 42,
    message: 'Downloading platform-tools…',
  },
}

export const PREVIEW_ADB_INSTALL_FAILED: AndroidDevicePreview = {
  ...PREVIEW_ADB_NOT_INSTALLED,
  installProgress: {
    phase: 'failed',
    percent: 0,
    error: 'Network error while downloading platform-tools',
  },
  error: 'Network error while downloading platform-tools',
}

export const PREVIEW_PROXY_ENABLED: AndroidDevicePreview = {
  ...PREVIEW_DEVICES_LISTED,
  deviceStates: states({
    'emulator-5554': { currentHttpProxy: 'proxy.lynx.example:7788' },
    'R58M90ABCDE': { currentHttpProxy: null },
  }),
}

export const PREVIEW_LAN_BLOCKED: AndroidDevicePreview = {
  ...PREVIEW_DEVICES_LISTED,
  localOnly: true,
}
