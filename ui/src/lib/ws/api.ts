import { WsOp, type WsEventFrame } from '@/lib/generated/ws/v1'
import type { CaptureFilter, GeneralSetting } from '@/lib/http/settings-types'
import type {
  ProjectsActiveSetPayload,
  ProjectsCreatePayload,
  ProjectsDeletePayload,
  ProjectsFileDto,
  ProjectsRenamePayload,
  RequestRuleDto,
  RulesDeletePayload,
  RulesDeleteResponse,
  RulesEnabledPayload,
  RulesGetPayload,
  RulesListPayload,
  RulesListResponse,
  RuleTemplatesResponse,
} from '@/lib/ws/rules-types'
import type { WsClient } from '@/lib/ws/client'

export interface CaptureStatusResponse {
  recordingStatus: string
}

export interface CaptureControlPayload {
  recording: boolean
}

export interface RequestDetailPayload {
  traceId: string
}

export interface RequestDetailValue {
  traceId: string
  status?: string | { Error?: string }
  request?: {
    method?: string
    url?: string
    headers?: Record<string, string>
    version?: string
    body?: unknown
  }
  response?: {
    status?: number
    headers?: Record<string, string>
    version?: string
    body?: unknown
  }
  timings?: Record<string, number | undefined>
}

export interface RequestDetailResponse {
  traceId: string
  detail?: RequestDetailValue | null
}

export interface CertificatePathResponse {
  path: string
}

export interface ComposeKeyValueRow {
  key: string
  value: string
  enabled: boolean
}

export type ComposeHttpMethod =
  | 'GET'
  | 'POST'
  | 'PUT'
  | 'PATCH'
  | 'DELETE'
  | 'HEAD'
  | 'OPTIONS'

export interface ComposeRequestPayload {
  method: ComposeHttpMethod
  url: string
  queryParams: ComposeKeyValueRow[]
  headers: ComposeKeyValueRow[]
  body: string
  timeout?: number
}

export interface ComposeResponse {
  status: number
  statusText: string
  headers: Record<string, string>
  body: string
  responseTime: number
  size: number
  errorMessage?: string
}

export interface WsApi {
  getCaptureStatus: () => Promise<CaptureStatusResponse>
  setCaptureControl: (payload: CaptureControlPayload) => Promise<unknown>
  getRequestDetail: (payload: RequestDetailPayload) => Promise<RequestDetailResponse>
  sendComposeRequest: (payload: ComposeRequestPayload) => Promise<ComposeResponse>
  subscribeRequestStream: () => Promise<unknown>
  unsubscribeRequestStream: () => Promise<unknown>
  getGeneralSetting: () => Promise<GeneralSetting>
  setGeneralSetting: (setting: GeneralSetting) => Promise<unknown>
  getCaptureFilter: () => Promise<CaptureFilter>
  setCaptureFilter: (filter: CaptureFilter) => Promise<unknown>
  getCertificatePath: () => Promise<CertificatePathResponse>
  listRules: (payload?: RulesListPayload) => Promise<RulesListResponse>
  getRule: (payload: RulesGetPayload) => Promise<RequestRuleDto>
  saveRule: (rule: RequestRuleDto) => Promise<RequestRuleDto>
  setRuleEnabled: (payload: RulesEnabledPayload) => Promise<RequestRuleDto>
  deleteRule: (payload: RulesDeletePayload) => Promise<RulesDeleteResponse>
  listRuleTemplates: () => Promise<RuleTemplatesResponse>
  listProjects: () => Promise<ProjectsFileDto>
  setActiveProject: (payload: ProjectsActiveSetPayload) => Promise<ProjectsFileDto>
  createProject: (payload: ProjectsCreatePayload) => Promise<ProjectsFileDto>
  renameProject: (payload: ProjectsRenamePayload) => Promise<ProjectsFileDto>
  deleteProject: (payload: ProjectsDeletePayload) => Promise<ProjectsFileDto>
  onEvent: (handler: (frame: WsEventFrame<Record<string, unknown>>) => void) => () => void
}

export const createWsApi = (client: WsClient): WsApi => {
  return {
    getCaptureStatus: () => client.call<CaptureStatusResponse>(WsOp.CaptureStatusGet),
    setCaptureControl: (payload) => client.call(WsOp.CaptureControlSet, payload),
    getRequestDetail: (payload) =>
      client.call<RequestDetailResponse, RequestDetailPayload>(
        WsOp.RequestDetailGet,
        payload,
      ),
    sendComposeRequest: (payload) =>
      client.call<ComposeResponse, ComposeRequestPayload>(WsOp.ComposeRequestSend, payload, ((payload.timeout || 30) + 10) * 1000),
    subscribeRequestStream: () => client.call(WsOp.RequestStreamSubscribe),
    unsubscribeRequestStream: () => client.call(WsOp.RequestStreamUnsubscribe),
    getGeneralSetting: () => client.call<GeneralSetting>(WsOp.SettingsGeneralGet),
    setGeneralSetting: (setting) => client.call(WsOp.SettingsGeneralSet, setting),
    getCaptureFilter: () => client.call<CaptureFilter>(WsOp.SettingsCaptureFilterGet),
    setCaptureFilter: (filter) => client.call(WsOp.SettingsCaptureFilterSet, filter),
    getCertificatePath: () => client.call<CertificatePathResponse>(WsOp.SettingsCertificatePathGet),
    listRules: (payload) =>
      client.call<RulesListResponse, RulesListPayload>(WsOp.RulesListGet, payload),
    getRule: (payload) => client.call<RequestRuleDto, RulesGetPayload>(WsOp.RulesGet, payload),
    saveRule: (rule) =>
      client.call<RequestRuleDto>(WsOp.RulesSaveSet, rule as unknown as Record<string, unknown>),
    setRuleEnabled: (payload) =>
      client.call<RequestRuleDto, RulesEnabledPayload>(WsOp.RulesEnabledSet, payload),
    deleteRule: (payload) =>
      client.call<RulesDeleteResponse, RulesDeletePayload>(WsOp.RulesDelete, payload),
    listRuleTemplates: () => client.call<RuleTemplatesResponse>(WsOp.RulesTemplatesGet),
    listProjects: () => client.call<ProjectsFileDto>(WsOp.ProjectsListGet),
    setActiveProject: (payload) =>
      client.call<ProjectsFileDto, ProjectsActiveSetPayload>(WsOp.ProjectsActiveSet, payload),
    createProject: (payload) =>
      client.call<ProjectsFileDto, ProjectsCreatePayload>(WsOp.ProjectsCreate, payload),
    renameProject: (payload) =>
      client.call<ProjectsFileDto, ProjectsRenamePayload>(WsOp.ProjectsRename, payload),
    deleteProject: (payload) =>
      client.call<ProjectsFileDto, ProjectsDeletePayload>(WsOp.ProjectsDelete, payload),
    onEvent: (handler) => client.onEvent(handler),
  }
}
