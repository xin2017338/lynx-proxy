import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import type { RuleDraft, RuleWorkbenchRuleItem } from '@/components/ui/rule-workbench'
import { createAction, createRuleDraft } from '@/components/ui/rule-workbench'
type PrimaryTabKey = 'rules' | 'assets' | 'compose' | 'device'
type SecondaryPaneKey = 'list' | 'editor'
import {
  cloneDraft,
  draftToRequestRule,
  parseRuleId,
  requestRuleToDraft,
  requestRuleToListItem,
  ruleIdToString,
} from '@/lib/ws/rules-mapper'
import { useWsConnectionStore } from './ws-connection.store'
import { WsOp } from '@/lib/generated/ws/v1'
import { projectIdFromName } from '@/lib/projects/project-id'
import type { RequestRuleDto, RulesListResponse, ProjectsFileDto, RuleProjectDto } from '@/lib/ws/rules-types'

export const useRulesStore = defineStore('rules', () => {
  const open = ref(false)
  const activePrimaryTab = ref<PrimaryTabKey>('rules')
  const rulesPane = ref<SecondaryPaneKey>('list')
  const assetsPane = ref<SecondaryPaneKey>('list')

  const rules = ref<RuleWorkbenchRuleItem[]>([])
  const projects = ref<RuleProjectDto[]>([{ id: 'default', name: 'Default' }])
  const activeProjectId = ref('default')
  const selectedRuleId = ref('')
  const ruleDraft = ref<RuleDraft | undefined>(undefined)
  const savedDraft = ref<RuleDraft | undefined>(undefined)

  const loading = ref(false)
  const saving = ref(false)
  const reordering = ref(false)
  const error = ref<string | null>(null)
  const creatingProject = ref(false)
  const createProjectError = ref<string | null>(null)

  const wsConnectionStore = useWsConnectionStore()

  const quickOverrideDraftIds = new Map<string, string>()
  let lastRulesDtoById = new Map<string, RequestRuleDto>()

  function buildListItemOptions() {
    const projectEnabledMap = new Map(
      projects.value.map(project => [project.id, project.enabled !== false]),
    )
    const projectNameMap = new Map(
      projects.value.map(project => [project.id, project.name]),
    )
    return { projectEnabledMap, projectNameMap }
  }

  function mapRulesToListItems(list: RequestRuleDto[]): RuleWorkbenchRuleItem[] {
    const options = buildListItemOptions()
    return list.map(rule => requestRuleToListItem(rule, options))
  }

  function resolveRuleProject(explicitProject?: string): string {
    if (activeProjectId.value !== 'default') return activeProjectId.value
    return explicitProject ?? 'default'
  }

  function stableDraftId(prefix: string, key: string): string {
    let hash = 0
    for (let i = 0; i < key.length; i += 1) {
      hash = ((hash << 5) - hash) + key.charCodeAt(i)
      hash |= 0
    }
    const token = Math.abs(hash).toString(36)
    return `draft-${prefix}-${token}`
  }

  const isDirty = computed(() => {
    if (!ruleDraft.value || !savedDraft.value) {
      return Boolean(ruleDraft.value && !savedDraft.value)
    }
    return JSON.stringify(ruleDraft.value) !== JSON.stringify(savedDraft.value)
  })

  async function refreshProjects() {
    const file = await wsConnectionStore.call<ProjectsFileDto>(WsOp.ProjectsListGet)
    projects.value = file?.projects?.length
      ? file.projects
      : [{ id: 'default', name: 'Default' }]
    activeProjectId.value = file?.activeProjectId || 'default'
  }

  async function refreshRules() {
    loading.value = true
    error.value = null
    try {
      const payload = activeProjectId.value === 'default'
        ? {}
        : { projectId: activeProjectId.value }
      const result = await wsConnectionStore.call<RulesListResponse>(
        WsOp.RulesListGet,
        payload,
      )
      const list = result?.rules ?? []
      lastRulesDtoById = new Map(list.map(rule => [ruleIdToString(rule.id), rule]))
      rules.value = mapRulesToListItems(list)
    } catch (err) {
      error.value = String(err)
      throw err
    } finally {
      loading.value = false
    }
  }

  async function openDrawer() {
    open.value = true
    await refreshProjects()
    await refreshRules()
  }

  async function openAndroidDrawer() {
    activePrimaryTab.value = 'device'
    open.value = true
    await refreshProjects()
    await refreshRules()
  }

  function closeDrawer() {
    open.value = false
  }

  async function loadRuleDraft(ruleId: string) {
    const numericId = parseRuleId(ruleId)
    if (numericId == null) {
      ruleDraft.value = createRuleDraft({ id: ruleId, project: resolveRuleProject() })
      savedDraft.value = cloneDraft(ruleDraft.value)
      return
    }

    loading.value = true
    error.value = null
    try {
      const rule = await wsConnectionStore.call<RequestRuleDto>(WsOp.RulesGet, { ruleId: numericId })
      ruleDraft.value = requestRuleToDraft(rule)
      savedDraft.value = cloneDraft(ruleDraft.value)
      selectedRuleId.value = ruleIdToString(rule.id)
    } catch (err) {
      error.value = String(err)
      throw err
    } finally {
      loading.value = false
    }
  }

  async function editRule(id: string) {
    selectedRuleId.value = id
    await loadRuleDraft(id)
  }

  function randomToken(): string {
    return Math.random().toString(36).slice(2, 9)
  }

  function newDraftId(prefix: string): string {
    return `draft-${prefix}-${randomToken()}`
  }

  function newActionId(): string {
    // Important: avoid `act-<number>` which draftToRequestRule() interprets as a persisted handler id.
    return `act-${randomToken()}`
  }

  async function duplicateRule(id: string) {
    await loadRuleDraft(id)
    if (!ruleDraft.value) {
      throw new Error('复制失败：未加载到规则草稿')
    }

    const source = cloneDraft(ruleDraft.value)
    const duplicated: RuleDraft = {
      ...source,
      id: newDraftId('copy'),
      name: `${source.name} 副本`,
      project: resolveRuleProject(source.project),
      actions: source.actions.map(action => ({
        ...action,
        id: newActionId(),
      })),
    }

    selectedRuleId.value = duplicated.id
    ruleDraft.value = duplicated
    savedDraft.value = cloneDraft(duplicated)
    rulesPane.value = 'editor'
  }

  async function openOrCreateQuickOverrideRule(input: {
    matchExpr: string
    seedBody: string
    isJson?: boolean
  }) {
    const matchExpr = input.matchExpr.trim()
    if (!matchExpr) {
      throw new Error('matchExpr 不能为空')
    }

    // Quick overrides always land in Default so saveRule won't rewrite project.
    try {
      await selectProject('default')
    } catch {
      // Still open the editor even if project switch fails.
    }

    open.value = true
    activePrimaryTab.value = 'rules'
    rulesPane.value = 'editor'

    const cachedId = quickOverrideDraftIds.get(matchExpr)
    const draftId = cachedId ?? stableDraftId('override', matchExpr)
    quickOverrideDraftIds.set(matchExpr, draftId)

    const nextDraft = createRuleDraft({
      id: draftId,
      name: `Override ${matchExpr}`,
      project: 'default',
      description: `quick-override:host+path\nmatchExpr=${matchExpr}\nformat=${input.isJson ? 'json' : 'text'}`,
      enabled: true,
      matchDsl: matchExpr,
      actions: [
        createAction({
          order: 1,
          type: 'modifyResponse',
          config: {
            modifyHeaders: [],
            modifyStatusCode: undefined,
            modifyBody: input.seedBody ?? '',
          },
        }),
      ],
    })

    // Keep list view refreshed so the drawer feels consistent.
    // This does not create server-side rules; it only syncs list content.
    try {
      await refreshRules()
    } catch {
      // ignore
    }

    selectedRuleId.value = draftId
    ruleDraft.value = nextDraft
    savedDraft.value = cloneDraft(nextDraft)
  }

  function createRule() {
    const draft = createRuleDraft({ project: resolveRuleProject() })
    ruleDraft.value = draft
    savedDraft.value = cloneDraft(draft)
    selectedRuleId.value = draft.id
    rulesPane.value = 'editor'
  }

  function updateRuleDraft(next: RuleDraft) {
    ruleDraft.value = next
    const id = selectedRuleId.value
    if (!id) return
    rules.value = rules.value.map((rule: RuleWorkbenchRuleItem) => (
      rule.id === id
        ? { ...rule, name: next.name, enabled: next.enabled }
        : rule
    ))
  }

  function goToRulesList() {
    rulesPane.value = 'list'
  }

  function clearRuleEditor() {
    selectedRuleId.value = ''
    ruleDraft.value = undefined
    savedDraft.value = undefined
  }

  async function deleteRule(id: string) {
    const wasSelected = selectedRuleId.value === id
    const ruleId = parseRuleId(id)
    error.value = null

    if (ruleId == null) {
      rules.value = rules.value.filter((rule: RuleWorkbenchRuleItem) => rule.id !== id)
      if (wasSelected) {
        clearRuleEditor()
        goToRulesList()
      }
      return
    }

    try {
      await wsConnectionStore.call(WsOp.RulesDelete, { ruleId })
      if (wasSelected) {
        clearRuleEditor()
        goToRulesList()
      }
      try {
        await refreshRules()
      } catch (refreshErr) {
        error.value = String(refreshErr)
      }
    } catch (err) {
      error.value = String(err)
      throw err
    }
  }

  async function saveRule(_id?: string) {
    if (!ruleDraft.value) {
      throw new Error('没有可保存的规则草稿')
    }
    saving.value = true
    error.value = null
    try {
      const project = resolveRuleProject(ruleDraft.value.project)
      const payload = draftToRequestRule(
        { ...ruleDraft.value, project },
        project,
      )
      const saved = await wsConnectionStore.call<RequestRuleDto>(
        WsOp.RulesSaveSet,
        payload as unknown as Record<string, unknown>,
      )
      ruleDraft.value = requestRuleToDraft(saved)
      savedDraft.value = cloneDraft(ruleDraft.value)
      selectedRuleId.value = ruleIdToString(saved.id)
      goToRulesList()
      try {
        await refreshRules()
      } catch (refreshErr) {
        error.value = String(refreshErr)
      }
    } catch (err) {
      error.value = String(err)
      throw err
    } finally {
      saving.value = false
    }
  }

  async function toggleRuleEnabled(id: string, enabled: boolean) {
    const ruleId = parseRuleId(id)
    if (ruleId == null) {
      rules.value = rules.value.map((rule: RuleWorkbenchRuleItem) => (
        rule.id === id ? { ...rule, enabled } : rule
      ))
      if (ruleDraft.value?.id === id) {
        ruleDraft.value = { ...ruleDraft.value, enabled }
      }
      return
    }

    error.value = null
    try {
      const updated = await wsConnectionStore.call<RequestRuleDto>(WsOp.RulesEnabledSet, {
        ruleId,
        enabled,
      })
      lastRulesDtoById.set(ruleIdToString(updated.id), updated)
      const listItem = requestRuleToListItem(updated, buildListItemOptions())
      rules.value = rules.value.map((rule: RuleWorkbenchRuleItem) => (
        rule.id === id
          ? { ...listItem, id }
          : rule
      ))
      if (selectedRuleId.value === id && ruleDraft.value) {
        ruleDraft.value = { ...ruleDraft.value, enabled }
        if (savedDraft.value) {
          savedDraft.value = { ...savedDraft.value, enabled }
        }
      }
    } catch (err) {
      error.value = String(err)
      throw err
    }
  }

  async function bulkToggleSelected(ids: string[], enabled: boolean) {
    error.value = null
    try {
      const numericIds = ids
        .map(id => parseRuleId(id))
        .filter((id): id is number => id != null)

      if (numericIds.length === 0) return

      // Call backend for each rule in parallel
      const updatePromises = numericIds.map(ruleId =>
        wsConnectionStore.call<RequestRuleDto>(WsOp.RulesEnabledSet, {
          ruleId,
          enabled,
        })
      )

      const updated = await Promise.all(updatePromises)

      // Update cache for all updated rules
      for (const updatedRule of updated) {
        lastRulesDtoById.set(ruleIdToString(updatedRule.id), updatedRule)
      }

      // Update local state
      const options = buildListItemOptions()
      rules.value = rules.value.map((rule) => {
        if (!ids.includes(rule.id)) return rule
        const dto = updated.find(item => ruleIdToString(item.id) === rule.id)
        return dto ? { ...requestRuleToListItem(dto, options), id: rule.id } : { ...rule, enabled }
      })

      // Update draft if currently editing one of the selected rules
      if (ruleDraft.value && ids.includes(ruleDraft.value.id)) {
        ruleDraft.value = { ...ruleDraft.value, enabled }
        if (savedDraft.value) {
          savedDraft.value = { ...savedDraft.value, enabled }
        }
      }
    } catch (err) {
      error.value = String(err)
      throw err
    }
  }

  function swapRules(idxA: number, idxB: number) {
    const next = [...rules.value]
    const tmp = next[idxA]
    next[idxA] = next[idxB]!
    next[idxB] = tmp!
    rules.value = next
  }

  async function persistCurrentListOrder() {
    // priority: 10000..0 where earlier items are higher priority
    const snapshot = [...rules.value]
    const updates: RequestRuleDto[] = []

    for (let i = 0; i < snapshot.length; i += 1) {
      const item = snapshot[i]!
      const numericId = parseRuleId(item.id)
      if (numericId == null) continue

      const dto = lastRulesDtoById.get(item.id)
      if (!dto) {
        // Fallback: list cache missing; fetch full rule to avoid accidental data loss.
        const fetched = await wsConnectionStore.call<RequestRuleDto>(WsOp.RulesGet, { ruleId: numericId })
        lastRulesDtoById.set(item.id, fetched)
      }

      const rule = lastRulesDtoById.get(item.id)
      if (!rule) continue

      const nextPriority = 10000 - i
      if (rule.priority === nextPriority) continue
      updates.push({ ...rule, priority: nextPriority })
    }

    // Persist sequentially to keep ordering stable.
    for (const dto of updates) {
      const saved = await wsConnectionStore.call<RequestRuleDto>(
        WsOp.RulesSaveSet,
        dto as unknown as Record<string, unknown>,
      )
      lastRulesDtoById.set(ruleIdToString(saved.id), saved)
    }
  }

  async function moveRuleUp(id: string) {
    const idx = rules.value.findIndex((r: RuleWorkbenchRuleItem) => r.id === id)
    if (idx <= 0) return
    if (reordering.value) return

    reordering.value = true
    error.value = null
    try {
      swapRules(idx, idx - 1)
      await persistCurrentListOrder()
      await refreshRules()
    } catch (err) {
      error.value = String(err)
      await refreshRules()
      throw err
    } finally {
      reordering.value = false
    }
  }

  async function moveRuleDown(id: string) {
    const idx = rules.value.findIndex((r: RuleWorkbenchRuleItem) => r.id === id)
    if (idx < 0 || idx >= rules.value.length - 1) return
    if (reordering.value) return

    reordering.value = true
    error.value = null
    try {
      swapRules(idx, idx + 1)
      await persistCurrentListOrder()
      await refreshRules()
    } catch (err) {
      error.value = String(err)
      await refreshRules()
      throw err
    } finally {
      reordering.value = false
    }
  }

  async function reorderRules(orderedIds: string[]) {
    if (reordering.value) return
    if (orderedIds.length === 0) return

    const byId = new Map(rules.value.map((rule: RuleWorkbenchRuleItem) => [rule.id, rule]))
    const next: RuleWorkbenchRuleItem[] = []
    for (const id of orderedIds) {
      const item = byId.get(id)
      if (item) next.push(item)
    }
    // Preserve any rules not present in orderedIds (shouldn't happen unless list is filtered).
    for (const item of rules.value) {
      if (!orderedIds.includes(item.id)) next.push(item)
    }

    reordering.value = true
    error.value = null
    try {
      rules.value = next
      await persistCurrentListOrder()
      await refreshRules()
    } catch (err) {
      error.value = String(err)
      await refreshRules()
      throw err
    } finally {
      reordering.value = false
    }
  }

  async function selectProject(projectId: string) {
    if (activeProjectId.value === projectId) return
    activeProjectId.value = projectId
    clearRuleEditor()
    goToRulesList()
    await wsConnectionStore.call(WsOp.ProjectsActiveSet, { projectId })
    await refreshRules()
  }

  async function createProjectWithName(name: string) {
    const trimmed = name.trim()
    if (!trimmed) return

    creatingProject.value = true
    createProjectError.value = null
    try {
      const id = projectIdFromName(trimmed)
      await wsConnectionStore.call(WsOp.ProjectsCreate, { id, name: trimmed })
      await refreshProjects()
      await selectProject(id)
    } catch (err) {
      createProjectError.value = String(err)
      throw err
    } finally {
      creatingProject.value = false
    }
  }

  async function renameProject(projectId: string, name: string) {
    const trimmed = name.trim()
    if (!trimmed || projectId === 'default') return

    creatingProject.value = true
    createProjectError.value = null
    try {
      await wsConnectionStore.call(WsOp.ProjectsRename, { projectId, name: trimmed })
      await refreshProjects()
    } catch (err) {
      createProjectError.value = String(err)
      throw err
    } finally {
      creatingProject.value = false
    }
  }

  async function moveRuleToProject(
    ruleId: string,
    targetProjectId: string,
    options?: { refresh?: boolean },
  ) {
    const numericId = parseRuleId(ruleId)
    if (numericId == null) return

    let dto = lastRulesDtoById.get(ruleId)
    if (!dto) {
      dto = await wsConnectionStore.call<RequestRuleDto>(WsOp.RulesGet, { ruleId: numericId })
      lastRulesDtoById.set(ruleId, dto)
    }

    const currentProject = dto.project ?? 'default'
    if (currentProject === targetProjectId) return

    error.value = null
    try {
      const saved = await wsConnectionStore.call<RequestRuleDto>(
        WsOp.RulesSaveSet,
        { ...dto, project: targetProjectId } as unknown as Record<string, unknown>,
      )
      lastRulesDtoById.set(ruleId, saved)
      if (options?.refresh !== false) {
        await refreshRules()
      }
    } catch (err) {
      error.value = String(err)
      throw err
    }
  }

  async function moveRulesToProject(ruleIds: string[], targetProjectId: string) {
    const uniqueIds = [...new Set(ruleIds)]
    if (uniqueIds.length === 0) return

    error.value = null
    try {
      for (const ruleId of uniqueIds) {
        await moveRuleToProject(ruleId, targetProjectId, { refresh: false })
      }
      await refreshRules()
    } catch (err) {
      error.value = String(err)
      throw err
    }
  }

  async function deleteActiveProject() {
    if (activeProjectId.value === 'default') return
    const current = projects.value.find(project => project.id === activeProjectId.value)
    const label = current?.name ?? activeProjectId.value
    if (!globalThis.confirm(`确定删除项目「${label}」？`)) return
    await wsConnectionStore.call(WsOp.ProjectsDelete, { projectId: activeProjectId.value })
    await refreshProjects()
    activeProjectId.value = 'default'
    await refreshRules()
  }

  async function toggleProjectEnabled(projectId: string, enabled: boolean) {
    if (projectId === 'default' && !enabled) return

    error.value = null
    try {
      const updated = await wsConnectionStore.call<RuleProjectDto>(WsOp.ProjectsEnabledSet, {
        projectId,
        enabled,
      })
      projects.value = projects.value.map(project => (
        project.id === projectId ? { ...project, ...updated } : project
      ))
      rules.value = mapRulesToListItems([...lastRulesDtoById.values()])
    } catch (err) {
      error.value = String(err)
      throw err
    }
  }

  return {
    open,
    activePrimaryTab,
    rulesPane,
    assetsPane,
    rules,
    projects,
    activeProjectId,
    selectedRuleId,
    ruleDraft,
    loading,
    saving,
    reordering,
    error,
    creatingProject,
    createProjectError,
    isDirty,
    openDrawer,
    openAndroidDrawer,
    closeDrawer,
    refreshProjects,
    refreshRules,
    selectProject,
    createProjectWithName,
    renameProject,
    moveRuleToProject,
    moveRulesToProject,
    deleteActiveProject,
    toggleProjectEnabled,
    editRule,
    duplicateRule,
    openOrCreateQuickOverrideRule,
    createRule,
    updateRuleDraft,
    saveRule,
    goToRulesList,
    toggleRuleEnabled,
    bulkToggleSelected,
    moveRuleUp,
    moveRuleDown,
    reorderRules,
    deleteRule,
  }
})
