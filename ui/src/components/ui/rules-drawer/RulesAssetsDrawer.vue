<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { computed, ref } from 'vue'
import { storeToRefs } from 'pinia'
import { ArrowLeft, X } from '@lucide/vue'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { useRulesStore } from '@/stores/modules/rules.store'
import { useComposeStore } from '@/stores/modules/compose.store'
import type { RuleDraft, RuleWorkbenchRuleItem } from '@/components/ui/rule-workbench'
import { createRuleDraft, getRuleValidationErrors, isRuleSaveDisabled } from '@/components/ui/rule-workbench'
import RuleWorkbench from '@/components/ui/rule-workbench/RuleWorkbench.vue'
import { ComposeWorkbench } from '@/components/ui/compose'
import DrawerTabs from './DrawerTabs.vue'
import ProjectSidebar from './ProjectSidebar.vue'
import RulesListView from './RulesListView.vue'
import RuleEditorToolbar from './RuleEditorToolbar.vue'
import { HorizontalSplitPanel } from '@/components/ui/split-panels'
import { AndroidDevicePanel, createWsAdbController } from '@/components/ui/android-device'
import type { AndroidDevicePreview } from '@/components/ui/android-device'
import { useWsConnectionStore } from '@/stores/modules/ws-connection.store'

export type PrimaryTabKey = 'rules' | 'assets' | 'compose' | 'device'
export type SecondaryPaneKey = 'list' | 'editor'

const props = withDefaults(defineProps<{
  open: boolean
  activePrimaryTab?: PrimaryTabKey
  rulesPane?: SecondaryPaneKey

  rules: RuleWorkbenchRuleItem[]
  selectedRuleId?: string
  ruleDraft?: RuleDraft

  dirty?: boolean
  loading?: boolean
  saving?: boolean
  /** Storybook: static Android panel without WebSocket */
  androidPreview?: AndroidDevicePreview
  class?: HTMLAttributes['class']
}>(), {
  activePrimaryTab: 'rules',
  rulesPane: 'list',
  selectedRuleId: '',
  ruleDraft: undefined,
  dirty: false,
  loading: false,
  saving: false,
})

const emit = defineEmits<{
  'update:open': [open: boolean]
  'update:activePrimaryTab': [tab: PrimaryTabKey]
  'update:rulesPane': [pane: SecondaryPaneKey]
  'update:selectedRuleId': [id: string]
  'update:ruleDraft': [draft: RuleDraft]

  'rules:create': []
  'rules:edit': [id: string]
  'rules:save': [id: string]
  'rules:toggle-enabled': [id: string, enabled: boolean]
  'rules:bulk-toggle': [ids: string[], enabled: boolean]
  'rules:reorder': [orderedIds: string[]]
  'rules:delete': [id: string]
}>()

const rulesSplitRatio = ref(26)

const tabs = computed(() => ([
  { key: 'rules', label: '规则' },
  { key: 'compose', label: 'Compose' },
  { key: 'device', label: 'Android' },
]))

const wsConnection = useWsConnectionStore()
const adbController = computed(() => (
  props.androidPreview ? undefined : createWsAdbController(wsConnection)
))

const rulesStore = useRulesStore()
const composeStore = useComposeStore()
const {
  activePrimaryTab: storeActivePrimaryTab,
  rulesPane: storeRulesPane,
  selectedRuleId: storeSelectedRuleId,
  ruleDraft: storeRuleDraft,
  reordering: storeReordering,
  projects: storeProjects,
  activeProjectId: storeActiveProjectId,
  creatingProject: storeCreatingProject,
  createProjectError: storeCreateProjectError,
} = storeToRefs(rulesStore)

/** Store is source of truth; props mirror v-model for Storybook. */
const rulesPaneDisplay = computed(() => storeRulesPane.value ?? props.rulesPane ?? 'list')
const activePrimaryTabDisplay = computed(() => storeActivePrimaryTab.value ?? props.activePrimaryTab ?? 'rules')

function setPrimaryTab(tab: PrimaryTabKey) {
  storeActivePrimaryTab.value = tab
  emit('update:activePrimaryTab', tab)
}

function setRulesPane(pane: SecondaryPaneKey) {
  storeRulesPane.value = pane
  emit('update:rulesPane', pane)
}

function close() {
  emit('update:open', false)
}

function openRulesList() {
  setPrimaryTab('rules')
  rulesStore.goToRulesList()
  setRulesPane('list')
}

function openRulesEditor() {
  setPrimaryTab('rules')
  setRulesPane('editor')
}

function onCreateRule() {
  emit('rules:create')
  openRulesEditor()
}

function onEditRule(id: string) {
  openRulesEditor()
  emit('update:selectedRuleId', id)
  emit('rules:edit', id)
}

function onSelectRule(id: string) {
  emit('update:selectedRuleId', id)
}

async function onDuplicateRule(id: string) {
  try {
    await rulesStore.duplicateRule(id)
  } catch {
    return
  }

  if (storeSelectedRuleId.value) {
    emit('update:selectedRuleId', storeSelectedRuleId.value)
  }
  if (storeRuleDraft.value) {
    emit('update:ruleDraft', storeRuleDraft.value)
  }
  openRulesEditor()
}

function onDeleteRule(id: string) {
  const name = props.rules.find(rule => rule.id === id)?.name ?? '该规则'
  if (!globalThis.confirm(`确定删除规则「${name}」？此操作不可撤销。`)) {
    return
  }
  emit('rules:delete', id)
}

async function handleToolbarSave() {
  const id = props.selectedRuleId
    || props.ruleDraft?.id
    || rulesStore.selectedRuleId
    || rulesStore.ruleDraft?.id
  if (!id) return
  try {
    await rulesStore.saveRule(id)
  } catch {
    return
  }
  openRulesList()
}

const ruleSaveDisabled = computed(() => {
  const draft = props.ruleDraft ?? createRuleDraft()
  return isRuleSaveDisabled({
    loading: props.loading,
    saving: props.saving,
    invalid: getRuleValidationErrors(draft).length > 0,
    hasSelection: !!props.selectedRuleId,
  })
})

const showDrawerBack = computed(() => (
  (activePrimaryTabDisplay.value === 'rules' && rulesPaneDisplay.value === 'editor')
))

function onDrawerBack() {
  openRulesList()
}

function onRuleDraftUpdate(next: RuleDraft) {
  const prev = props.ruleDraft
  emit('update:ruleDraft', next)
  if (!props.selectedRuleId || !prev) return
  if (next.enabled !== prev.enabled) {
    emit('rules:toggle-enabled', props.selectedRuleId, next.enabled)
  }
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="props.open"
      class="fixed inset-0 z-50 flex justify-end bg-black/40"
      @click.self="close"
    >
      <section
        role="dialog"
        aria-modal="true"
        aria-label="规则与动作资产"
        :class="cn(
          'flex h-full w-full max-w-[980px] flex-col bg-background shadow-2xl',
          props.class,
        )"
      >
        <DrawerTabs
          :model-value="activePrimaryTabDisplay"
          :tabs="tabs"
          trailing
          @update:model-value="setPrimaryTab($event as any)"
        >
          <template #trailing>
            <Button
              v-if="showDrawerBack"
              variant="ghost"
              size="sm"
              class="h-8 shrink-0 px-2 text-xs"
              aria-label="返回列表"
              @click="onDrawerBack"
            >
              <ArrowLeft class="h-3.5 w-3.5" />
              返回
            </Button>
            <Button size="icon-sm" variant="ghost" class="h-8 w-8" @click="close" aria-label="关闭抽屉">
              <X class="h-4 w-4" />
            </Button>
          </template>
        </DrawerTabs>

        <div
          class="min-h-0 flex-1 overflow-x-hidden overflow-y-hidden"
          :class="(activePrimaryTabDisplay === 'compose' || activePrimaryTabDisplay === 'device') ? 'px-3 pb-3 pt-2' : 'p-3'"
        >
          <div
            v-if="activePrimaryTabDisplay === 'device'"
            class="flex h-full min-h-0 flex-col overflow-hidden"
          >
            <AndroidDevicePanel
              :preview="props.androidPreview"
              :controller="adbController"
              class="h-full"
            />
          </div>

          <div v-else-if="activePrimaryTabDisplay === 'compose'" class="flex h-full min-h-0 flex-col">
            <ComposeWorkbench
              v-model:draft="composeStore.draft"
              :response="composeStore.response"
              :loading="composeStore.loading"
              :error="composeStore.error"
              embedded
              class="h-full"
              @send="composeStore.send"
              @reset="composeStore.reset"
            />
          </div>

          <!-- Rules -->
          <div v-else-if="activePrimaryTabDisplay === 'rules'" class="flex h-full min-h-0 flex-col">
            <HorizontalSplitPanel
              v-model="rulesSplitRatio"
              class="min-h-0 flex-1"
              :min-left-px="180"
              :min-right-px="320"
            >
              <template #left>
                <ProjectSidebar
                  :projects="storeProjects"
                  :active-project-id="storeActiveProjectId"
                  :saving="storeCreatingProject"
                  :error="storeCreateProjectError"
                  class="h-full"
                  @select="rulesStore.selectProject"
                  @create="rulesStore.createProjectWithName"
                  @rename="rulesStore.renameProject"
                  @move-rules="rulesStore.moveRulesToProject"
                  @toggle-enabled="rulesStore.toggleProjectEnabled"
                />
              </template>

              <template #right>
                <div v-if="rulesPaneDisplay === 'list'" class="flex h-full min-h-0 flex-col overflow-hidden">
                  <RulesListView
                    :rules="props.rules"
                    :selected-rule-id="props.selectedRuleId"
                    :reordering="storeReordering"
                    :show-all-projects="storeActiveProjectId === 'default'"
                    class="min-h-0 flex-1"
                    @create="onCreateRule"
                    @edit="onEditRule"
                    @duplicate="onDuplicateRule"
                    @select="onSelectRule"
                    @toggle-enabled="(id, enabled) => emit('rules:toggle-enabled', id, enabled)"
                    @bulk-toggle="(ids, enabled) => emit('rules:bulk-toggle', ids, enabled)"
                    @reorder="ids => emit('rules:reorder', ids)"
                    @delete="onDeleteRule"
                  />
                </div>

                <div v-else class="flex h-full min-h-0 flex-col overflow-hidden">
                  <RuleEditorToolbar
                    :draft="props.ruleDraft"
                    :saving="props.saving"
                    :save-disabled="ruleSaveDisabled"
                    @update:draft="onRuleDraftUpdate"
                    @save="handleToolbarSave"
                  />

                  <RuleWorkbench
                    :rules="props.rules"
                    :draft="props.ruleDraft"
                    :selected-rule-id="props.selectedRuleId"
                    :dirty="props.dirty"
                    :loading="props.loading"
                    :saving="props.saving"
                    embedded
                    :show-list="false"
                    class="min-h-0 flex-1"
                    @update:draft="onRuleDraftUpdate"
                    @update:selected-rule-id="emit('update:selectedRuleId', $event)"
                    @save="handleToolbarSave"
                  />
                </div>
              </template>
            </HorizontalSplitPanel>
          </div>
        </div>
      </section>
    </div>
  </Teleport>
</template>

