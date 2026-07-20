export interface GeneralSetting {
  maxLogSize: number
  language: string
  /** When true (default), Network list hides CONNECT tunnel rows. */
  hideConnectTunnels?: boolean
}

export interface DomainFilter {
  domain: string
  enabled: boolean
  port: number
}

export interface CaptureFilter {
  enabled: boolean
  includeDomains: DomainFilter[]
  excludeDomains: DomainFilter[]
}

export const DEFAULT_DOMAIN_FILTER: DomainFilter = {
  domain: '',
  enabled: true,
  port: 443,
}

export const DEFAULT_CAPTURE_FILTER: CaptureFilter = {
  enabled: true,
  includeDomains: [],
  excludeDomains: [],
}

export const MAX_LOG_SIZE_MIN = 60
export const MAX_LOG_SIZE_MAX = 6000
