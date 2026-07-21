<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { computed, ref, watch } from 'vue'
import { CheckCircle2, CircleDashed, ListFilter, TriangleAlert } from '@lucide/vue'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Switch } from '@/components/ui/switch'
import ActionPipelineBuilder from './ActionPipelineBuilder.vue'
import MatchDslEditor from './MatchDslEditor.vue'
import { getRuleValidationErrors } from './match-validation'
import { createRuleDraft } from './types'
import type { RuleDraft } from './types'
import { getRuleSaveStatusLabel, isRuleSaveDisabled } from './save-status'
import { sortRulesForDisplay } from '@/lib/ws/rules-mapper'

export type RuleMobilePane = 'list' | 'editor'

export interface RuleWorkbenchRuleItem {
  id: string
  name: string
  enabled: boolean
  effectiveEnabled?: boolean
  priority: number
  summary?: string
  forwardUrl?: string
  projectId?: string
  projectName?: string
  createdAt?: number | null
  updatedAt?: number | null
  state?: 'draft' | 'valid' | 'invalid'
}

interface RuleWorkbenchProps {
  rules: RuleWorkbenchRuleItem[]
  draft?: RuleDraft
  selectedRuleId?: string
  mobilePane?: RuleMobilePane
  loading?: boolean
  dirty?: boolean
  saving?: boolean
  embedded?: boolean
  showList?: boolean
  class?: HTMLAttributes['class']
}

const props = withDefaults(defineProps<RuleWorkbenchProps>(), {
  selectedRuleId: '',
  mobilePane: 'editor',
  loading: false,
  dirty: false,
  saving: false,
  embedded: false,
  showList: true,
})

const emit = defineEmits<{
  'update:draft': [draft: RuleDraft]
  'update:selectedRuleId': [id: string]
  'update:mobilePane': [pane: RuleMobilePane]
  'save': [id: string]
  'toggle-enabled': [id: string, enabled: boolean]
}>()

const searchTerm = ref('')

const selectedRuleIdLocal = ref(props.selectedRuleId)
const mobilePaneLocal = ref<RuleMobilePane>(props.mobilePane)
const draftLocal = ref<RuleDraft>(props.draft ?? createRuleDraft())

watch(() => props.selectedRuleId, next => {
  selectedRuleIdLocal.value = next
})

watch(() => props.mobilePane, next => {
  mobilePaneLocal.value = next
})

watch(() => props.draft, next => {
  if (!next) return
  draftLocal.value = next
})

watch(draftLocal, (next) => {
  emit('update:draft', next)
}, { deep: true })

const filteredRules = computed(() => {
  const keyword = searchTerm.value.trim().toLowerCase()
  if (!keyword) return props.rules
  return props.rules.filter(rule => (
    rule.name.toLowerCase().includes(keyword)
    || (rule.summary ?? '').toLowerCase().includes(keyword)
    || (rule.forwardUrl ?? '').toLowerCase().includes(keyword)
    || (rule.projectName ?? '').toLowerCase().includes(keyword)
  ))
})

const displayedRules = computed(() => sortRulesForDisplay(filteredRules.value, 'updatedAt'))

function isEffectivelyEnabled(rule: RuleWorkbenchRuleItem): boolean {
  return rule.effectiveEnabled ?? rule.enabled
}

function ruleSwitchTitle(rule: RuleWorkbenchRuleItem): string | undefined {
  if (rule.enabled && rule.effectiveEnabled === false) {
    return '项目已禁用，规则暂不生效'
  }
  return undefined
}

const selectedRule = computed(() => {
  return props.rules.find(rule => rule.id === selectedRuleIdLocal.value) ?? displayedRules.value[0]
})

const validationErrors = computed(() => getRuleValidationErrors(draftLocal.value))
const isInvalid = computed(() => validationErrors.value.length > 0)

const isSaveDisabled = computed(() => isRuleSaveDisabled({
  loading: props.loading,
  saving: props.saving,
  invalid: isInvalid.value,
  hasSelection: !!selectedRule.value,
}))

const statusLabel = computed(() => getRuleSaveStatusLabel({
  loading: props.loading,
  invalid: isInvalid.value,
  dirty: props.dirty,
  valid: !isInvalid.value,
}))

function ruleStateLabel(state?: RuleWorkbenchRuleItem['state']) {
  if (state === 'invalid') return '无效'
  if (state === 'valid') return '有效'
  return '草稿'
}

function updateSelectedRule(id: string) {
  selectedRuleIdLocal.value = id
  emit('update:selectedRuleId', id)
}

function updateMobilePane(pane: RuleMobilePane) {
  mobilePaneLocal.value = pane
  emit('update:mobilePane', pane)
}

function requestSave() {
  if (!selectedRule.value) return
  emit('save', selectedRule.value.id)
}

function updateMatchDsl(next: string) {
  draftLocal.value = {
    ...draftLocal.value,
    matchDsl: next,
  }
}

function updateActions(next: RuleDraft['actions']) {
  draftLocal.value = {
    ...draftLocal.value,
    actions: next,
  }
}

function ruleStateClass(state?: RuleWorkbenchRuleItem['state']) {
  if (state === 'invalid') return 'text-destructive'
  if (state === 'valid') return 'text-emerald-600'
  return 'text-muted-foreground'
}
</script>

<template>
  <section
    :class="cn(
      props.embedded
        ? 'flex h-full min-h-0 w-full flex-col overflow-hidden bg-transparent'
        : 'flex min-h-[520px] w-full flex-col overflow-hidden rounded-lg border border-border bg-background',
      props.class,
    )"
  >
    <header v-if="!props.embedded" class="flex flex-wrap items-center justify-between gap-2 border-b border-border px-3 py-2">
      <div class="flex flex-wrap items-center gap-2">
        <span
          class="inline-flex items-center gap-1 rounded-sm border border-border bg-muted/30 px-2 py-1 text-[11px] font-medium"
          :class="isInvalid ? 'text-destructive' : 'text-foreground'"
        >
          <TriangleAlert v-if="isInvalid" class="h-3.5 w-3.5" />
          <CheckCircle2 v-else-if="!props.dirty" class="h-3.5 w-3.5 text-emerald-600" />
          <CircleDashed v-else class="h-3.5 w-3.5" />
          {{ statusLabel }}
        </span>

        <Button size="default" class="h-8 px-3 text-xs" :disabled="isSaveDisabled" @click="requestSave">
          {{ props.saving ? '保存中...' : '保存' }}
        </Button>
      </div>
    </header>

    <div v-if="props.showList" class="bg-muted/20 px-2 pt-2 md:hidden">
      <div role="tablist" aria-label="移动端面板" class="flex border-b border-border/50">
        <button
          type="button"
          role="tab"
          aria-controls="rule-pane-list"
          :aria-selected="mobilePaneLocal === 'list'"
          class="-mb-px h-8 flex-1 border-b-2 px-2 text-xs font-medium transition-colors"
          :class="mobilePaneLocal === 'list'
            ? 'border-primary text-foreground'
            : 'border-transparent text-muted-foreground hover:text-foreground'"
          @click="updateMobilePane('list')"
        >
          列表
        </button>
        <button
          type="button"
          role="tab"
          aria-controls="rule-pane-editor"
          :aria-selected="mobilePaneLocal === 'editor'"
          class="-mb-px h-8 flex-1 border-b-2 px-2 text-xs font-medium transition-colors"
          :class="mobilePaneLocal === 'editor'
            ? 'border-primary text-foreground'
            : 'border-transparent text-muted-foreground hover:text-foreground'"
          @click="updateMobilePane('editor')"
        >
          编辑
        </button>
      </div>
    </div>

    <div
      class="min-h-0 flex-1"
      :class="props.showList
        ? 'grid grid-cols-1 md:grid-cols-[280px_minmax(0,1fr)]'
        : 'flex min-h-0 flex-col'"
    >
      <aside
        v-if="props.showList"
        id="rule-pane-list"
        class="flex min-h-0 flex-col bg-muted/20 md:border-r md:border-border/55"
        :class="mobilePaneLocal !== 'list' ? 'hidden md:flex' : 'flex'"
      >
        <div class="px-3 py-2">
          <label class="sr-only" for="rule-workbench-search">搜索规则</label>
          <div class="relative">
            <ListFilter class="pointer-events-none absolute left-2 top-1.5 h-3.5 w-3.5 text-muted-foreground" />
            <input
              id="rule-workbench-search"
              v-model="searchTerm"
              type="search"
              class="h-7 w-full rounded-sm border border-input bg-background pl-7 pr-2 text-xs text-foreground outline-none ring-ring transition-colors placeholder:text-muted-foreground focus:ring-1"
              placeholder="搜索规则、转发 URL"
            >
          </div>
        </div>

        <ul class="flex-1 space-y-1 overflow-auto p-2">
          <li v-if="displayedRules.length === 0" class="rounded-sm border border-dashed border-border p-3 text-xs text-muted-foreground">
            没有匹配当前筛选条件的规则。
          </li>
          <li v-for="rule in displayedRules" :key="rule.id">
            <div
              class="flex items-start gap-2 rounded-md px-2.5 py-2 transition-colors"
              :class="[
                rule.id === selectedRule?.id
                  ? 'bg-primary/10 shadow-sm ring-1 ring-primary/35'
                  : 'bg-background/80 hover:bg-accent/50',
                !isEffectivelyEnabled(rule) && 'opacity-70',
              ]"
            >
              <div class="pt-0.5" @click.stop>
                <Switch
                  :checked="rule.enabled"
                  :title="ruleSwitchTitle(rule)"
                  :aria-label="rule.enabled ? `禁用 ${rule.name}` : `启用 ${rule.name}`"
                  @update:checked="emit('toggle-enabled', rule.id, $event)"
                />
              </div>
              <button
                type="button"
                class="min-w-0 flex-1 text-left"
                @click="updateSelectedRule(rule.id)"
              >
                <div class="flex items-center justify-between gap-2">
                  <div class="flex min-w-0 items-center gap-1.5">
                    <p class="truncate text-xs font-semibold text-foreground">{{ rule.name }}</p>
                    <span
                      v-if="rule.projectName"
                      class="shrink-0 rounded-sm bg-muted px-1 py-0.5 text-[10px] text-muted-foreground"
                    >
                      {{ rule.projectName }}
                    </span>
                  </div>
                  <span class="shrink-0 text-[10px] font-medium" :class="ruleStateClass(rule.state)">
                    {{ ruleStateLabel(rule.state) }}
                  </span>
                </div>
                <p class="mt-1 line-clamp-2 text-[11px] leading-4 text-muted-foreground">
                  {{ rule.summary || '暂无摘要。' }}
                </p>
                <p
                  v-if="rule.forwardUrl"
                  class="mt-0.5 truncate font-mono text-[10px] text-primary/80"
                  :title="rule.forwardUrl"
                >
                  → {{ rule.forwardUrl }}
                </p>
              </button>
            </div>
          </li>
        </ul>
      </aside>

      <main
        id="rule-pane-editor"
        class="flex min-h-0 flex-1 flex-col bg-muted/10"
        :class="props.showList
          ? (mobilePaneLocal !== 'editor' ? 'hidden md:flex' : 'flex')
          : 'flex'"
      >
        <div class="flex min-h-0 flex-1 flex-col">
          <div
            v-if="!props.embedded"
            class="border-b border-border px-3 py-2"
          >
            <p class="truncate text-xs font-semibold text-foreground">
              {{ selectedRule?.name || '未选择规则' }}
            </p>
          </div>

          <div class="min-h-0 flex-1 overflow-x-hidden overflow-y-auto px-3 py-3 scrollbar-gutter-stable">
            <div class="grid min-w-0 gap-3">
              <slot
                name="editor"
                :rule="selectedRule"
                :draft="draftLocal"
              >
                <MatchDslEditor
                  :model-value="draftLocal.matchDsl"
                  @update:model-value="updateMatchDsl"
                />

                <ActionPipelineBuilder
                  :model-value="draftLocal.actions"
                  @update:model-value="updateActions"
                />
              </slot>
            </div>
          </div>
        </div>
      </main>
    </div>
  </section>
</template>
