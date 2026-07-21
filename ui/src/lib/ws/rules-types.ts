export interface CaptureRuleDto {
  id?: number | null
  matchExpr: string
}

export type HandlerRuleTypeDto =
  | { type: 'block'; statusCode?: number; reason?: string }
  | { type: 'modifyRequest'; modifyHeaders?: Record<string, string>; modifyBody?: string; modifyMethod?: string; modifyUrl?: string }
  | { type: 'modifyResponse'; modifyHeaders?: Record<string, string>; modifyBody?: string; modifyMethod?: string; modifyStatusCode?: number }
  | { type: 'localFile'; filePath: string; contentType?: string; statusCode?: number }
  | { type: 'proxyForward'; targetScheme?: string; targetAuthority?: string; targetPath?: string }
  | { type: 'htmlScriptInjector'; content?: string; injectionPosition?: string }
  | { type: 'delay'; delayMs: number; varianceMs?: number; delayType?: 'beforeRequest' | 'afterRequest' | 'both' }
  | { type: 'throttle'; preset?: string; downloadKbps?: number; uploadKbps?: number; latencyMs?: number }

export interface HandlerRuleDto {
  id?: number | null
  handlerType: HandlerRuleTypeDto
  executionOrder: number
  enabled: boolean
}

export interface RequestRuleDto {
  id?: number | null
  project?: string
  name: string
  description?: string | null
  enabled: boolean
  priority: number
  capture: CaptureRuleDto
  handlers: HandlerRuleDto[]
  createdAt?: number | null
  updatedAt?: number | null
}

export interface RulesListResponse {
  rules: RequestRuleDto[]
}

export interface RuleProjectDto {
  id: string
  name: string
  enabled?: boolean
}

export interface ProjectsFileDto {
  activeProjectId: string
  projects: RuleProjectDto[]
}

export interface ProjectsCreatePayload {
  id: string
  name: string
}

export interface ProjectsRenamePayload {
  projectId: string
  name: string
}

export interface ProjectsDeletePayload {
  projectId: string
}

export interface ProjectsActiveSetPayload {
  projectId: string
}

export interface ProjectsEnabledPayload {
  projectId: string
  enabled: boolean
}

export interface RulesListPayload {
  projectId?: string
}

export interface RuleTemplatesResponse {
  templates: HandlerRuleDto[]
}

export interface RulesGetPayload {
  ruleId: number
}

export interface RulesEnabledPayload {
  ruleId: number
  enabled: boolean
}

export interface RulesDeletePayload {
  ruleId: number
}

export interface RulesDeleteResponse {
  ruleId: number
}
