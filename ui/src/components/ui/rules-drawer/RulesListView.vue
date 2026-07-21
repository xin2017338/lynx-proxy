<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { computed, ref, watch } from 'vue'
import Draggable from 'vuedraggable'
import Sortable from 'sortablejs'
import { ArrowDownUp, GripVertical, ListFilter, Plus } from '@lucide/vue'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Switch } from '@/components/ui/switch'
import type { RuleWorkbenchRuleItem } from '@/components/ui/rule-workbench'
import { type RuleListSortMode, sortRulesForDisplay } from '@/lib/ws/rules-mapper'
import { drawerEmptyStateClass, drawerFilterChipClass, drawerListItemClass, drawerSearchInputClass } from './drawer-styles'
import {
  clearDraggingRuleIds,
  draggingRuleIds,
  externalRuleDropHandled,
  setRuleDragData,
} from './rule-drag'
import { useRuleListSelection } from './useRuleListSelection'

interface SortableItemEvent {
  oldIndex?: number
  from?: HTMLElement
  item?: HTMLElement
}

const props = withDefaults(defineProps<{
  rules: RuleWorkbenchRuleItem[]
  selectedRuleId?: string
  reordering?: boolean
  showAllProjects?: boolean
  class?: HTMLAttributes['class']
}>(), {
  selectedRuleId: '',
  reordering: false,
  showAllProjects: false,
})

const emit = defineEmits<{
  create: []
  edit: [id: string]
  duplicate: [id: string]
  delete: [id: string]
  select: [id: string]
  'toggle-enabled': [id: string, enabled: boolean]
  'bulk-toggle': [ids: string[], enabled: boolean]
  reorder: [orderedIds: string[]]
}>()

const searchTerm = ref('')
const sortMode = ref<RuleListSortMode>('updatedAt')
const localRules = ref<RuleWorkbenchRuleItem[]>([])
const listRootRef = ref<HTMLElement | null>(null)

const {
  selectedIds,
  selectedCount,
  isSelected,
  clearSelection,
  pruneSelection,
  toggleSelected,
  setSelected,
  isAllSelected,
  isAnySelected,
  idsForDrag,
} = useRuleListSelection()

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

const displayedRules = computed(() => sortRulesForDisplay(filteredRules.value, sortMode.value))

const visibleRuleIds = computed(() => displayedRules.value.map(r => r.id))
const allVisibleSelected = computed(() => isAllSelected(visibleRuleIds.value))
const anyVisibleSelected = computed(() => isAnySelected(visibleRuleIds.value))
const someVisibleSelected = computed(() => anyVisibleSelected.value && !allVisibleSelected.value)

const orderedRuleIds = computed(() => localRules.value.map(rule => rule.id))

const allSelectedEnabled = computed(() => {
  if (selectedIds.value.size === 0) return false
  const selectedRules = displayedRules.value.filter(rule => selectedIds.value.has(rule.id))
  return selectedRules.length > 0 && selectedRules.every(rule => rule.enabled)
})

function handleBulkToggle() {
  if (selectedIds.value.size === 0) return
  const newState = !allSelectedEnabled.value
  emit('bulk-toggle', Array.from(selectedIds.value), newState)
}

watch(displayedRules, (next) => {
  localRules.value = [...next]
  pruneSelection(next.map(rule => rule.id))
}, { immediate: true })

const reorderDisabled = computed(() => (
  props.reordering
  || searchTerm.value.trim().length > 0
  || sortMode.value !== 'priority'
  || props.showAllProjects
))

const dragHandleDisabled = computed(() => props.reordering)

function toggleSelectAllVisible() {
  const ids = visibleRuleIds.value
  if (ids.length === 0) return
  if (allVisibleSelected.value) {
    clearSelection()
  }
  else {
    setSelected(ids)
  }
}

function clearSortableMultiSelect(rootEl: HTMLElement) {
  rootEl.querySelectorAll('.sortable-selected').forEach((el) => {
    Sortable.utils.deselect(el as HTMLElement)
  })
}

function syncSortableMultiSelect(rootEl: HTMLElement, ruleIds: string[]) {
  clearSortableMultiSelect(rootEl)
  for (const id of ruleIds) {
    const el = rootEl.querySelector(`li[data-rule-id="${id}"]`)
    if (el) {
      Sortable.utils.select(el as HTMLElement)
    }
  }
}

function prepareDragSelection(draggedId: string, rootEl: HTMLElement | null | undefined) {
  const ids = idsForDrag(draggedId, orderedRuleIds.value)
  draggingRuleIds.value = ids
  if (rootEl) {
    syncSortableMultiSelect(rootEl, ids)
  }
  return ids
}

function onSortableChoose(evt: SortableItemEvent) {
  if (dragHandleDisabled.value) return
  const draggedId = evt.item?.dataset.ruleId
  if (!draggedId) return
  prepareDragSelection(draggedId, evt.item?.parentElement ?? evt.from)
  externalRuleDropHandled.value = false
}

function onSortableStart(evt: SortableItemEvent) {
  if (dragHandleDisabled.value) return
  const index = evt.oldIndex
  if (index == null || index < 0) return
  const rule = localRules.value[index]
  if (!rule) return
  prepareDragSelection(rule.id, evt.from)
}

function onDragEnd() {
  const skipReorder = externalRuleDropHandled.value
  clearDraggingRuleIds()
  externalRuleDropHandled.value = false
  if (listRootRef.value) {
    clearSortableMultiSelect(listRootRef.value)
  }
  if (reorderDisabled.value || skipReorder) {
    if (skipReorder) {
      clearSelection()
    }
    return
  }
  const ordered = localRules.value.map(r => r.id)
  emit('reorder', ordered)
}

function setDragData(dataTransfer: DataTransfer, dragEl: HTMLElement) {
  if (dragHandleDisabled.value) return
  const draggedId = dragEl.dataset.ruleId
  if (!draggedId) return
  const ids = idsForDrag(draggedId, orderedRuleIds.value)
  draggingRuleIds.value = ids
  setRuleDragData(dataTransfer, ids)
}

function onRuleRowClick(rule: RuleWorkbenchRuleItem, index: number, ev: MouseEvent) {
  void index
  void ev
  toggleSelected(rule.id)
}

function onListBackgroundClick() {
  clearSelection()
}

function dragHandleTitle(ruleId: string) {
  if (dragHandleDisabled.value) {
    return '正在保存排序，请稍候'
  }
  if (reorderDisabled.value) {
    return '不可拖拽排序，可拖到左侧项目移动归属'
  }
  const count = selectedCount.value
  if (count > 1 && isSelected(ruleId)) {
    return `拖拽排序；将移动 ${count} 条规则`
  }
  return '拖拽排序；拖到左侧项目可移动归属'
}

function ruleStateLabel(state?: RuleWorkbenchRuleItem['state']) {
  if (state === 'invalid') return '无效'
  if (state === 'valid') return '有效'
  return '草稿'
}

function ruleStateClass(state?: RuleWorkbenchRuleItem['state']) {
  if (state === 'invalid') return 'text-destructive'
  if (state === 'valid') return 'text-emerald-600'
  return 'text-muted-foreground'
}

function isEffectivelyEnabled(rule: RuleWorkbenchRuleItem): boolean {
  return rule.effectiveEnabled ?? rule.enabled
}

function ruleSwitchTitle(rule: RuleWorkbenchRuleItem): string | undefined {
  if (rule.enabled && rule.effectiveEnabled === false) {
    return '项目已禁用，规则暂不生效'
  }
  return undefined
}

function sharesForwardUrlWithPrevious(index: number): boolean {
  if (sortMode.value !== 'forwardUrl' || index <= 0) return false
  const current = localRules.value[index]?.forwardUrl
  const previous = localRules.value[index - 1]?.forwardUrl
  return !!current && current === previous
}

function startsForwardUrlGroup(index: number): boolean {
  if (sortMode.value !== 'forwardUrl') return false
  const current = localRules.value[index]?.forwardUrl
  if (!current) return false
  if (index === 0) return true
  return localRules.value[index - 1]?.forwardUrl !== current
}
</script>

<template>
  <section :class="cn('flex h-full min-h-0 flex-col overflow-hidden', props.class)">
    <div class="flex items-center justify-between gap-2 px-2 pb-2 pt-2">
      <div class="relative flex-1">
        <ListFilter class="pointer-events-none absolute left-2 top-1.5 h-3.5 w-3.5 text-muted-foreground" />
        <input
          v-model="searchTerm"
          type="text"
          inputmode="search"
          :class="[drawerSearchInputClass, 'pl-7 pr-2']"
          placeholder="搜索规则、转发 URL"
        >
      </div>

      <Button variant="outline" size="default" class="px-2.5" @click="emit('create')">
        <Plus class="h-3.5 w-3.5" />
        新建
      </Button>
    </div>

    <div class="flex flex-wrap items-center gap-2 px-2 pb-2">
      <ArrowDownUp class="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
      <button
        type="button"
        :class="drawerFilterChipClass(sortMode === 'updatedAt')"
        @click="sortMode = 'updatedAt'"
      >
        更新时间
      </button>
      <button
        type="button"
        :class="drawerFilterChipClass(sortMode === 'createdAt')"
        @click="sortMode = 'createdAt'"
      >
        创建时间
      </button>
      <button
        type="button"
        :class="drawerFilterChipClass(sortMode === 'priority')"
        @click="sortMode = 'priority'"
      >
        优先级
      </button>
      <button
        type="button"
        :class="drawerFilterChipClass(sortMode === 'forwardUrl')"
        @click="sortMode = 'forwardUrl'"
      >
        转发 URL
      </button>
    </div>

    <div class="flex items-center justify-between gap-2 px-2 pb-2">
      <div class="flex min-w-0 items-center gap-2">
        <button
          type="button"
          class="inline-flex items-center gap-2 rounded-sm px-1 py-1 text-xs text-muted-foreground hover:text-foreground disabled:opacity-40"
          :disabled="visibleRuleIds.length === 0"
          @click="toggleSelectAllVisible"
        >
          <input
            type="checkbox"
            class="h-3.5 w-3.5 accent-primary"
            :checked="allVisibleSelected"
            :indeterminate.prop="someVisibleSelected"
            @click.stop
            @change="toggleSelectAllVisible"
          >
          <span class="truncate">全选（当前可见）</span>
        </button>

        <span
          class="text-xs text-muted-foreground"
          :class="selectedCount > 0 ? 'opacity-100' : 'pointer-events-none opacity-0'"
        >
          已选 {{ selectedCount }}
        </span>
      </div>

      <div v-if="selectedCount > 0" class="flex shrink-0 items-center gap-2">
        <span class="text-xs text-muted-foreground">{{ allSelectedEnabled ? '批量禁用' : '批量启用' }}</span>
        <Switch
          :checked="allSelectedEnabled"
          @update:checked="handleBulkToggle"
        />
      </div>
    </div>

    <div class="min-h-0 flex-1 overflow-auto px-2 pb-2 scrollbar-gutter-stable" @click.self="onListBackgroundClick">
      <div v-if="displayedRules.length === 0" :class="drawerEmptyStateClass">
        没有匹配当前筛选条件的规则。
      </div>

      <Draggable
        v-else
        ref="listRootRef"
        v-model="localRules"
        item-key="id"
        tag="ul"
        class="space-y-2"
        :disabled="dragHandleDisabled"
        :sort="!reorderDisabled"
        handle=".drag-handle"
        :multi-drag="true"
        selected-class="sortable-selected"
        :set-data="setDragData"
        :animation="180"
        ghost-class="rule-drag-ghost"
        chosen-class="rule-drag-chosen"
        drag-class="rule-drag-dragging"
        @choose="onSortableChoose"
        @start="onSortableStart"
        @end="onDragEnd"
      >
        <template #item="{ element: rule, index }">
          <li
            :data-rule-id="rule.id"
            :class="cn(
              startsForwardUrlGroup(index) && 'pt-1',
            )"
          >
            <div
              :class="cn(
                drawerListItemClass(isSelected(rule.id)),
                'cursor-default',
                !isEffectivelyEnabled(rule) && 'opacity-70',
                sharesForwardUrlWithPrevious(index) && 'border-t border-primary/20 bg-primary/[0.03]',
                startsForwardUrlGroup(index) && rule.forwardUrl && 'ring-1 ring-primary/15',
              )"
              @click="onRuleRowClick(rule, index, $event)"
            >
              <div class="flex items-start gap-2">
                <button
                  type="button"
                  class="drag-handle mt-0.5 inline-flex h-6 w-6 items-center justify-center rounded-sm text-muted-foreground hover:text-foreground"
                  :class="dragHandleDisabled ? 'cursor-not-allowed opacity-50' : 'cursor-grab active:cursor-grabbing'"
                  :disabled="dragHandleDisabled"
                  :aria-label="`拖拽排序或移动 ${rule.name}`"
                  :title="dragHandleTitle(rule.id)"
                  @click.stop
                >
                  <GripVertical class="h-4 w-4" />
                </button>

                <div class="pt-0.5" @click.stop>
                  <Switch
                    :checked="rule.enabled"
                    :title="ruleSwitchTitle(rule)"
                    :aria-label="rule.enabled ? `禁用 ${rule.name}` : `启用 ${rule.name}`"
                    @update:checked="emit('toggle-enabled', rule.id, $event)"
                  />
                </div>

                <div class="min-w-0 flex-1 select-none text-left">
                  <div class="flex items-center justify-between gap-2">
                    <div class="flex min-w-0 items-center gap-1.5">
                      <p class="truncate text-xs font-semibold text-foreground">{{ rule.name }}</p>
                      <span
                        v-if="showAllProjects && rule.projectName"
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
                </div>
              </div>

              <div class="mt-2 flex justify-end gap-1">
                <Button variant="ghost" size="sm" class="h-7 px-2 text-xs" @click.stop="emit('edit', rule.id)">
                  编辑
                </Button>
                <Button variant="ghost" size="sm" class="h-7 px-2 text-xs" @click.stop="emit('duplicate', rule.id)">
                  复制
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  class="h-7 px-2 text-xs text-destructive hover:text-destructive"
                  :aria-label="`删除 ${rule.name}`"
                  @click.stop="emit('delete', rule.id)"
                >
                  删除
                </Button>
              </div>
            </div>
          </li>
        </template>
      </Draggable>
    </div>
  </section>
</template>

<style scoped>
.rule-drag-ghost {
  opacity: 0.45;
}

.rule-drag-chosen {
  opacity: 0.9;
}

.rule-drag-dragging {
  opacity: 0.85;
}

:deep(li.sortable-selected > div) {
  border-color: hsl(var(--primary) / 0.45);
  background-color: hsl(var(--primary) / 0.05);
}
</style>
