/* eslint-disable */
// Generated from crates/lynx-core/protocol/ws.v1.asyncapi.yaml. Do not edit manually.

export const WS_PROTOCOL_VERSION = 'v1' as const

export type WsProtocolVersion = typeof WS_PROTOCOL_VERSION

export const WsFrameKind = {
  Request: 'request',
  Response: 'response',
  Event: 'event',
  Error: 'error',
  Ping: 'ping',
  Pong: 'pong',
} as const

export type WsFrameKind = (typeof WsFrameKind)[keyof typeof WsFrameKind]

export const WsOp = {
  SystemPing: 'system.ping',
  CaptureStatusGet: 'capture.status.get',
  CaptureControlSet: 'capture.control.set',
  RequestDetailGet: 'request.detail.get',
  RequestStreamSubscribe: 'request.stream.subscribe',
  RequestStreamUnsubscribe: 'request.stream.unsubscribe',
  ComposeRequestSend: 'compose.request.send',
  SettingsGeneralGet: 'settings.general.get',
  SettingsGeneralSet: 'settings.general.set',
  SettingsCaptureFilterGet: 'settings.captureFilter.get',
  SettingsCaptureFilterSet: 'settings.captureFilter.set',
  SettingsCertificatePathGet: 'settings.certificate.path.get',
  RulesListGet: 'rules.list.get',
  RulesGet: 'rules.get',
  RulesSaveSet: 'rules.save.set',
  RulesEnabledSet: 'rules.enabled.set',
  RulesDelete: 'rules.delete',
  RulesTemplatesGet: 'rules.templates.get',
  ProjectsListGet: 'projects.list.get',
  ProjectsActiveSet: 'projects.active.set',
  ProjectsCreate: 'projects.create',
  ProjectsRename: 'projects.rename',
  ProjectsDelete: 'projects.delete',
  ProjectsEnabledSet: 'projects.enabled.set',
  CaptureRulesFocusListGet: 'capture.rules.focus.list.get',
  CaptureRulesIgnoreListGet: 'capture.rules.ignore.list.get',
  CaptureRulesFocusUpsert: 'capture.rules.focus.upsert',
  CaptureRulesIgnoreUpsert: 'capture.rules.ignore.upsert',
  CaptureRulesFocusDelete: 'capture.rules.focus.delete',
  CaptureRulesIgnoreDelete: 'capture.rules.ignore.delete',
  CaptureRulesFocusEnabledSet: 'capture.rules.focus.enabled.set',
  CaptureRulesIgnoreEnabledSet: 'capture.rules.ignore.enabled.set',
  DeviceAdbStatusGet: 'device.adb.status.get',
  DeviceAdbInstall: 'device.adb.install',
  DeviceAdbInstallProgressGet: 'device.adb.install.progress.get',
  DeviceAdbDevicesList: 'device.adb.devices.list',
  DeviceAdbProxyStateGet: 'device.adb.proxy.state.get',
  DeviceAdbProxyEnable: 'device.adb.proxy.enable',
  DeviceAdbProxyDisable: 'device.adb.proxy.disable',
  NetworkTrafficFilterHistoryGet: 'network.trafficFilter.history.get',
  NetworkTrafficFilterHistoryAppend: 'network.trafficFilter.history.append',
  NetworkTrafficFilterHistoryClear: 'network.trafficFilter.history.clear',
  CaptureStatusChanged: 'capture.status.changed',
  RequestStart: 'request.start',
  RequestBody: 'request.body',
  RequestEnd: 'request.end',
  ResponseStart: 'response.start',
  ResponseBody: 'response.body',
  ResponseEnd: 'response.end',
  WebsocketMessage: 'websocket.message',
  WebsocketError: 'websocket.error',
  WebsocketEnd: 'websocket.end',
  SystemError: 'system.error',
} as const

export type WsRequestOp =
  | 'system.ping'
  | 'capture.status.get'
  | 'capture.control.set'
  | 'request.detail.get'
  | 'request.stream.subscribe'
  | 'request.stream.unsubscribe'
  | 'compose.request.send'
  | 'settings.general.get'
  | 'settings.general.set'
  | 'settings.captureFilter.get'
  | 'settings.captureFilter.set'
  | 'settings.certificate.path.get'
  | 'rules.list.get'
  | 'rules.get'
  | 'rules.save.set'
  | 'rules.enabled.set'
  | 'rules.delete'
  | 'rules.templates.get'
  | 'projects.list.get'
  | 'projects.active.set'
  | 'projects.create'
  | 'projects.rename'
  | 'projects.delete'
  | 'projects.enabled.set'
  | 'capture.rules.focus.list.get'
  | 'capture.rules.ignore.list.get'
  | 'capture.rules.focus.upsert'
  | 'capture.rules.ignore.upsert'
  | 'capture.rules.focus.delete'
  | 'capture.rules.ignore.delete'
  | 'capture.rules.focus.enabled.set'
  | 'capture.rules.ignore.enabled.set'
  | 'device.adb.status.get'
  | 'device.adb.install'
  | 'device.adb.install.progress.get'
  | 'device.adb.devices.list'
  | 'device.adb.proxy.state.get'
  | 'device.adb.proxy.enable'
  | 'device.adb.proxy.disable'
  | 'network.trafficFilter.history.get'
  | 'network.trafficFilter.history.append'
  | 'network.trafficFilter.history.clear'

export type WsEventOp =
  | 'capture.status.changed'
  | 'request.start'
  | 'request.body'
  | 'request.end'
  | 'response.start'
  | 'response.body'
  | 'response.end'
  | 'websocket.message'
  | 'websocket.error'
  | 'websocket.end'
  | 'system.error'

export interface WsErrorPayload {
  code: string
  message: string
  details?: Record<string, unknown>
}

export interface WsBaseFrame {
  version: WsProtocolVersion
  kind: WsFrameKind
  id: string
  op: string
  timestamp: number
}

export interface WsRequestFrame<TPayload = Record<string, unknown>>
  extends WsBaseFrame {
  kind: typeof WsFrameKind.Request
  payload?: TPayload
}

export interface WsResponseFrame<TPayload = Record<string, unknown>>
  extends WsBaseFrame {
  kind: typeof WsFrameKind.Response
  payload?: TPayload
}

export interface WsEventFrame<TPayload = Record<string, unknown>> extends WsBaseFrame {
  kind: typeof WsFrameKind.Event
  payload?: TPayload
}

export interface WsErrorFrame extends WsBaseFrame {
  kind: typeof WsFrameKind.Error
  error: WsErrorPayload
}

export interface WsPingFrame extends WsBaseFrame {
  kind: typeof WsFrameKind.Ping
}

export interface WsPongFrame extends WsBaseFrame {
  kind: typeof WsFrameKind.Pong
}

export type WsClientFrame<TPayload = Record<string, unknown>> =
  | WsRequestFrame<TPayload>
  | WsPingFrame

export type WsServerFrame<TPayload = Record<string, unknown>> =
  | WsResponseFrame<TPayload>
  | WsEventFrame<TPayload>
  | WsErrorFrame
  | WsPongFrame

export interface WsRequestOptions {
  timeoutMs?: number
}

export const isWsResponseFrame = (
  frame: WsServerFrame,
): frame is WsResponseFrame<Record<string, unknown>> => {
  return frame.kind === WsFrameKind.Response
}

export const isWsEventFrame = (
  frame: WsServerFrame,
): frame is WsEventFrame<Record<string, unknown>> => {
  return frame.kind === WsFrameKind.Event
}

export const isWsErrorFrame = (frame: WsServerFrame): frame is WsErrorFrame => {
  return frame.kind === WsFrameKind.Error
}

export const isWsEventOp = (op: string): op is WsEventOp => {
  return op in WsOp
}
