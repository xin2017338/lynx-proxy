import type { RequestViewMode } from '@/components/ui/network-panels'
import type { CaptureFilter, GeneralSetting } from '@/lib/http/settings-types'
import { DEFAULT_CAPTURE_FILTER } from '@/lib/http/settings-types'
import {
  DEFAULT_SPLIT_RATIO,
  DEFAULT_TABLE_SPLIT_RATIO,
} from '@/stores/modules/settings.store'

export type NetworkViewMode = RequestViewMode

export interface NetworkPreferencesState {
  viewMode: NetworkViewMode
  splitRatio: number
  tableSplitRatio: number
  streamEnabled: boolean
}

export const DEFAULT_NETWORK_PREFERENCES: NetworkPreferencesState = {
  viewMode: 'table',
  splitRatio: DEFAULT_SPLIT_RATIO,
  tableSplitRatio: DEFAULT_TABLE_SPLIT_RATIO,
  streamEnabled: true,
}

export interface SettingsPanelPreview {
  general: GeneralSetting
  capture: CaptureFilter
  certPath: string
  baseAddresses: string[]
}

export const DEFAULT_SETTINGS_PREVIEW: SettingsPanelPreview = {
  general: {
    maxLogSize: 5000,
    language: 'zh-CN',
    hideConnectTunnels: true,
  },
  capture: {
    ...DEFAULT_CAPTURE_FILTER,
    includeDomains: [
      { domain: '*.example.com', port: 443, enabled: true },
    ],
  },
  certPath: '/var/lynx/ca/root_ca.pem',
  baseAddresses: ['127.0.0.1:8080'],
}
