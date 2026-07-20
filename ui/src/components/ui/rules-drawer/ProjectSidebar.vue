<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { computed, nextTick, ref, watch } from 'vue'
import { Plus } from '@lucide/vue'
import { cn } from '@/lib/utils'
import { apiStudioTreeRenameInputClass } from '@/components/ui/api-studio/api-studio-styles'
import { Switch } from '@/components/ui/switch'
import type { RuleProjectDto } from '@/lib/ws/rules-types'
import { drawerListItemClass } from './drawer-styles'
import {
  clearDraggingRuleIds,
  externalRuleDropHandled,
  isRuleDragEvent,
  readDraggedRuleIds,
} from './rule-drag'

const props = defineProps<{
  projects: RuleProjectDto[]
  activeProjectId: string
  saving?: boolean
  error?: string | null
  class?: HTMLAttributes['class']
}>()

const emit = defineEmits<{
  select: [projectId: string]
  create: [name: string]
  rename: [projectId: string, name: string]
  moveRules: [ruleIds: string[], projectId: string]
  'toggle-enabled': [projectId: string, enabled: boolean]
}>()

const dropTargetProjectId = ref<string | null>(null)
const createInputRef = ref<HTMLInputElement | null>(null)
const renameInputRef = ref<HTMLInputElement | null>(null)

const isCreating = ref(false)
const createName = ref('')
const renamingProjectId = ref<string | null>(null)
const renameName = ref('')

const sortedProjects = computed(() => (
  [...props.projects].sort((a, b) => {
    if (a.id === 'default') return -1
    if (b.id === 'default') return 1
    return a.name.localeCompare(b.name)
  })
))

const isEditing = computed(() => isCreating.value || renamingProjectId.value != null)

function projectItemClass(active: boolean, dropTarget = false, disabled = false) {
  return cn(
    drawerListItemClass(active || dropTarget),
    dropTarget && 'border-primary bg-primary/5',
    'w-full text-left text-xs font-medium transition-colors',
    !active && !dropTarget && 'text-muted-foreground hover:text-foreground',
    active && 'text-foreground',
    disabled && 'opacity-60',
  )
}

function isProjectEnabled(project: RuleProjectDto): boolean {
  return project.enabled !== false
}

function isRuleDrag(ev: DragEvent) {
  return isRuleDragEvent(ev)
}

function onAsideDragOver(ev: DragEvent) {
  if (!isRuleDrag(ev)) return
  ev.preventDefault()
  if (ev.dataTransfer) {
    ev.dataTransfer.dropEffect = 'move'
  }
}

function canAcceptRuleDrop(projectId: string) {
  return !isEditing.value
    && !props.saving
    && projectId !== props.activeProjectId
}

function onProjectDragOver(projectId: string, ev: DragEvent) {
  if (!isRuleDrag(ev) || !canAcceptRuleDrop(projectId)) return
  ev.preventDefault()
  if (ev.dataTransfer) {
    ev.dataTransfer.dropEffect = 'move'
  }
}

function onProjectDragEnter(projectId: string, ev: DragEvent) {
  if (!isRuleDrag(ev) || !canAcceptRuleDrop(projectId)) return
  dropTargetProjectId.value = projectId
}

function onProjectDragLeave(projectId: string, ev: DragEvent) {
  if (dropTargetProjectId.value !== projectId) return
  const related = ev.relatedTarget as Node | null
  if (related && (ev.currentTarget as HTMLElement).contains(related)) return
  dropTargetProjectId.value = null
}

function onProjectDrop(projectId: string, ev: DragEvent) {
  dropTargetProjectId.value = null
  if (!isRuleDrag(ev) || !canAcceptRuleDrop(projectId)) return
  const ruleIds = readDraggedRuleIds(ev)
  if (ruleIds.length === 0) return
  ev.preventDefault()
  ev.stopPropagation()
  externalRuleDropHandled.value = true
  clearDraggingRuleIds()
  emit('moveRules', ruleIds, projectId)
}

function focusInput(target: typeof createInputRef | typeof renameInputRef) {
  nextTick(() => {
    const el = target.value
    if (!el) return
    el.focus()
    el.select()
  })
}

function startCreating() {
  if (isEditing.value || props.saving) return
  isCreating.value = true
  createName.value = ''
  renamingProjectId.value = null
  focusInput(createInputRef)
}

function cancelCreating() {
  isCreating.value = false
  createName.value = ''
}

function startRenaming(project: RuleProjectDto) {
  if (project.id === 'default' || isEditing.value || props.saving) return
  renamingProjectId.value = project.id
  renameName.value = project.name
  isCreating.value = false
  focusInput(renameInputRef)
}

function cancelRenaming() {
  renamingProjectId.value = null
  renameName.value = ''
}

function selectProject(projectId: string) {
  if (isEditing.value || props.saving) return
  if (projectId === props.activeProjectId) return
  emit('select', projectId)
}

function submitCreate() {
  const trimmed = createName.value.trim()
  if (!trimmed || props.saving) return
  emit('create', trimmed)
}

function onCreateKeydown(ev: KeyboardEvent) {
  if (ev.key === 'Enter') {
    ev.preventDefault()
    submitCreate()
  }
  else if (ev.key === 'Escape') {
    ev.preventDefault()
    cancelCreating()
  }
}

function onCreateBlur() {
  if (!createName.value.trim()) {
    cancelCreating()
  }
}

function submitRename(projectId: string) {
  const trimmed = renameName.value.trim()
  const project = props.projects.find(item => item.id === projectId)
  if (!trimmed || !project || trimmed === project.name || props.saving) {
    cancelRenaming()
    return
  }
  emit('rename', projectId, trimmed)
}

function onRenameKeydown(ev: KeyboardEvent, projectId: string) {
  if (ev.key === 'Enter') {
    ev.preventDefault()
    submitRename(projectId)
  }
  else if (ev.key === 'Escape') {
    ev.preventDefault()
    cancelRenaming()
  }
}

function onRenameBlur(projectId: string) {
  const project = props.projects.find(item => item.id === projectId)
  const trimmed = renameName.value.trim()
  if (!trimmed || !project || trimmed === project.name) {
    cancelRenaming()
    return
  }
  submitRename(projectId)
}

let wasSaving = false
watch(() => props.saving, (saving) => {
  if (wasSaving && !saving && !props.error) {
    if (isCreating.value) cancelCreating()
    if (renamingProjectId.value != null) cancelRenaming()
  }
  wasSaving = Boolean(saving)
})

watch(() => props.error, (error) => {
  if (!error) return
  if (isCreating.value) focusInput(createInputRef)
  if (renamingProjectId.value != null) focusInput(renameInputRef)
})
</script>

<template>
  <aside
    role="navigation"
    aria-label="规则项目"
    :class="cn('flex h-full min-h-0 flex-col overflow-hidden border-r border-border/60 bg-card', props.class)"
  >
    <div
      class="min-h-0 flex-1 overflow-auto px-2 py-2"
      @dragover="onAsideDragOver"
    >
      <ul class="space-y-1">
        <li v-for="project in sortedProjects" :key="project.id">
          <div
            v-if="renamingProjectId === project.id"
            :class="projectItemClass(activeProjectId === project.id)"
          >
            <input
              ref="renameInputRef"
              v-model="renameName"
              type="text"
              :class="[apiStudioTreeRenameInputClass, 'w-full']"
              :disabled="saving"
              @keydown="onRenameKeydown($event, project.id)"
              @blur="onRenameBlur(project.id)"
              @click.stop
              @mousedown.stop
            >
          </div>
          <div
            v-else
            :class="projectItemClass(
              activeProjectId === project.id,
              dropTargetProjectId === project.id,
              !isProjectEnabled(project),
            )"
          >
            <div class="flex items-center gap-2">
              <button
                type="button"
                class="min-w-0 flex-1 text-left"
                :disabled="isEditing || saving"
                :title="project.id === 'default'
                  ? project.name
                  : `${project.name}（双击重命名）${!isProjectEnabled(project) ? ' · 已禁用' : ''}`"
                @click="selectProject(project.id)"
                @dblclick.prevent="startRenaming(project)"
                @dragover="onProjectDragOver(project.id, $event)"
                @dragenter="onProjectDragEnter(project.id, $event)"
                @dragleave="onProjectDragLeave(project.id, $event)"
                @drop="onProjectDrop(project.id, $event)"
              >
                <span class="block truncate">{{ project.name }}</span>
              </button>
              <div
                v-if="project.id !== 'default'"
                class="shrink-0"
                @click.stop
              >
                <Switch
                  :checked="isProjectEnabled(project)"
                  :aria-label="isProjectEnabled(project) ? `禁用项目 ${project.name}` : `启用项目 ${project.name}`"
                  @update:checked="emit('toggle-enabled', project.id, $event)"
                />
              </div>
            </div>
          </div>
        </li>

        <li v-if="isCreating">
          <div :class="projectItemClass(true)">
            <input
              ref="createInputRef"
              v-model="createName"
              type="text"
              placeholder="项目名称"
              :class="[apiStudioTreeRenameInputClass, 'w-full']"
              :disabled="saving"
              @keydown="onCreateKeydown"
              @blur="onCreateBlur"
              @click.stop
              @mousedown.stop
            >
          </div>
        </li>
      </ul>
    </div>

    <div class="flex shrink-0 items-center border-t border-border/60 px-2 py-1.5">
      <button
        type="button"
        class="inline-flex size-7 items-center justify-center rounded-sm text-muted-foreground transition-colors hover:bg-muted/50 hover:text-foreground disabled:pointer-events-none disabled:opacity-40"
        aria-label="新建项目"
        :disabled="isEditing || saving"
        @click="startCreating"
      >
        <Plus class="size-4" />
      </button>
    </div>

    <p v-if="error" class="shrink-0 px-2 pb-2 text-xs text-destructive">
      {{ error }}
    </p>
  </aside>
</template>
