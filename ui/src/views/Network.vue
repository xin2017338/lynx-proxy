<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { Button, NetworkRequestDetail } from '@/components'
import { type TrafficRecord } from '@/components/ui/request-tree'
import { HorizontalSplitPanel, VerticalSplitPanel } from '@/components/ui/split-panels'
import { CaptureRulesPopover, NetworkRequestPanel, TrafficMatchFilterInput, type RequestViewMode } from '@/components/ui/network-panels'
import { RulesAssetsDrawer } from '@/components/ui/rules-drawer'
import { useTrafficFilterHistory } from '@/composables/useTrafficFilterHistory'
import { useTrafficMatchFilter } from '@/composables/useTrafficMatchFilter'
import { useCaptureStore, useGeneralSettingsStore, useRequestStreamStore, useRulesStore, useSettingsStore, useWsConnectionStore } from '@/stores'
import { Disc2, ListTree, PlugZap, BrushCleaning, Sheet, Scale, Crosshair } from '@lucide/vue'
import { cn } from '@/lib/utils'

const captureStore = useCaptureStore()
const requestStreamStore = useRequestStreamStore()
const wsConnectionStore = useWsConnectionStore()
const settingsStore = useSettingsStore()
const generalSettingsStore = useGeneralSettingsStore()
const rulesStore = useRulesStore()
const {
  open: rulesDrawerOpen,
  activePrimaryTab: rulesDrawerPrimaryTab,
  rulesPane,
  selectedRuleId,
  ruleDraft,
} = storeToRefs(rulesStore)

const {
  viewMode: requestViewMode,
  splitRatio,
  tableSplitRatio,
  streamEnabled,
  trafficFilterDsl,
} = storeToRefs(settingsStore)

const { hideConnectTunnels } = storeToRefs(generalSettingsStore)

const {
  entries: trafficFilterHistory,
  push: pushTrafficFilterHistory,
  clear: clearTrafficFilterHistory,
} = useTrafficFilterHistory()

const visibleTrafficRecords = computed(() => {
  const records = requestStreamStore.trafficRecords
  if (!hideConnectTunnels.value) {
    return records
  }
  return records.filter(record => record.method?.toUpperCase() !== 'CONNECT')
})

const {
  filteredRecords,
  filterState,
  filterError,
  applyFilter,
} = useTrafficMatchFilter({
  filterDsl: trafficFilterDsl,
  trafficRecords: visibleTrafficRecords,
  recordsByTrace: computed(() => requestStreamStore.recordsByTrace),
})

const isDesktop = ref(false)

let mediaQueryList: MediaQueryList | null = null
let detachMediaQuery: (() => void) | null = null

const bindDesktopMediaQuery = () => {
  if (typeof window === 'undefined') {
    return
  }

  mediaQueryList = window.matchMedia('(min-width: 1024px)')

  const updateDesktopState = () => {
    isDesktop.value = mediaQueryList?.matches ?? false
  }

  updateDesktopState()

  mediaQueryList.addEventListener('change', updateDesktopState)
  detachMediaQuery = () => {
    mediaQueryList?.removeEventListener('change', updateDesktopState)
  }
}

const selectedTraceId = computed({
  get: () => requestStreamStore.selectedId,
  set: (value: string | undefined) => {
    requestStreamStore.select(value)
  },
})

const startStream = async () => {
  await requestStreamStore.start()
  await requestStreamStore.subscribe()
}

const stopStream = async () => {
  await requestStreamStore.unsubscribe()
}

const setStreamEnabled = async (enabled: boolean) => {
  streamEnabled.value = enabled

  if (enabled) {
    await startStream()
  } else {
    await stopStream()
  }
}

const setRecording = async (enabled: boolean) => {
  await captureStore.setRecording(enabled)
}

const handleRequestSelect = (request: TrafficRecord) => {
  requestStreamStore.select(request.id)
}

const handleMatchedRuleOpen = async (rule: { ruleId: string | number }) => {
  const id = String(rule?.ruleId ?? '')
  if (!id) return

  rulesPane.value = 'editor'
  await rulesStore.openDrawer()
  try {
    await rulesStore.editRule(id)
  } catch {
    // ignore load failure; drawer仍会打开，方便用户手动查找
  }
}

const handleViewModeChange = (mode: RequestViewMode) => {
  requestViewMode.value = mode
}

const handleFilterSubmit = async (value: string) => {
  await applyFilter(value)
  if (filterState.value === 'valid') {
    await pushTrafficFilterHistory(value)
  }
}

const handleClearFilterHistory = async () => {
  await clearTrafficFilterHistory()
}

watch(streamEnabled, async (enabled, previous) => {
  if (previous === undefined) {
    return
  }

  if (enabled) {
    await startStream()
  } else {
    await stopStream()
  }
})

watch(
  () => wsConnectionStore.isConnected,
  async (connected, wasConnected) => {
    if (!connected || wasConnected || !streamEnabled.value) {
      return
    }

    await startStream()
  },
)

onMounted(async () => {
  settingsStore.hydrate()
  if (trafficFilterDsl.value.trim()) {
    await applyFilter()
  }
  bindDesktopMediaQuery()
  captureStore.handleServerEvent()
  await captureStore.refreshStatus()

  if (streamEnabled.value) {
    await startStream()
  }
})

onBeforeUnmount(async () => {
  detachMediaQuery?.()
  detachMediaQuery = null
  mediaQueryList = null

  await requestStreamStore.stop()
  captureStore.dispose()
})
</script>

<template>
  <div class="flex h-full min-h-0 flex-col  overflow-hidden">
    <div class="flex items-center justify-between px-1">
      <div class="inline-flex min-w-0 flex-1 items-center gap-1">
        <Button
          size="icon-sm"
          variant="ghost"
          title="Table View"
          :aria-pressed="requestViewMode === 'table'"
          @click="handleViewModeChange('table')"
        >
          <Sheet :class="cn('h-4 w-4', requestViewMode === 'table' ? 'text-primary' : 'text-muted-foreground/60')" />
        </Button>
        <Button
          size="icon-sm"
          variant="ghost"
          title="Tree View"
          :aria-pressed="requestViewMode === 'tree'"
          @click="handleViewModeChange('tree')"
        >
          <ListTree :class="cn('h-4 w-4', requestViewMode === 'tree' ? 'text-primary' : 'text-muted-foreground/60')" />
        </Button>

        <TrafficMatchFilterInput
          v-model="trafficFilterDsl"
          :filter-state="filterState"
          :filter-error="filterError"
          :history="trafficFilterHistory"
          class="relative ml-1 max-w-[360px] flex-1"
          @submit="handleFilterSubmit"
          @clear-history="handleClearFilterHistory"
        />
      </div>

      <div class="flex items-center gap-3">
        <Button
          size="icon-sm"
          variant="ghost"
          :title="streamEnabled ? 'Disable Stream' : 'Enable Stream'"
          :aria-pressed="streamEnabled"
          @click="setStreamEnabled(!streamEnabled)"
        >
          <PlugZap :class="cn('h-4 w-4', streamEnabled ? 'text-primary' : 'text-muted-foreground/60')" />
        </Button>

        <Button
          size="icon-sm"
          variant="ghost"
          :title="captureStore.isRecording ? 'Stop Capture' : 'Start Capture'"
          :disabled="captureStore.loading"
          :aria-pressed="captureStore.isRecording"
          @click="setRecording(!captureStore.isRecording)"
        >
          <Disc2 :class="cn('h-4 w-4', captureStore.isRecording ? 'text-primary' : 'text-muted-foreground/60')" />
        </Button>

        <Button size="icon-sm" variant="ghost" title="Clear Requests" @click="requestStreamStore.clear">
          <BrushCleaning class="h-4 w-4 text-muted-foreground/70" />
        </Button>

        <CaptureRulesPopover>
          <Button
            size="icon-sm"
            variant="ghost"
            title="Focus / Ignore"
          >
            <Crosshair class="h-4 w-4 text-muted-foreground/70" />
          </Button>
        </CaptureRulesPopover>

        <Button
          size="icon-sm"
          variant="ghost"
          title="Rules"
          :aria-pressed="rulesStore.open"
          @click="rulesStore.openDrawer()"
        >
          <Scale :class="cn('h-4 w-4', rulesStore.open ? 'text-primary' : 'text-muted-foreground/60')" />
        </Button>
      </div>
    </div>

    <VerticalSplitPanel
      v-if="requestViewMode === 'table'"
      v-model="tableSplitRatio"
      class="flex-1"
      :min-top-px="220"
      :min-bottom-px="220"
    >
      <template #top>
        <section class="h-full rounded-md bg-card/60">
          <NetworkRequestPanel
            v-model="selectedTraceId"
            :requests="filteredRecords"
            :view-mode="requestViewMode"
            @select="handleRequestSelect"
          />
        </section>
      </template>

      <template #bottom>
        <section class="h-full rounded-md bg-card/60">
          <NetworkRequestDetail
            :record="requestStreamStore.selectedRecord"
            class="h-full"
            @rule:open="handleMatchedRuleOpen"
          />
        </section>
      </template>
    </VerticalSplitPanel>

    <HorizontalSplitPanel
      v-else
      v-model="splitRatio"
      :enabled="isDesktop"
      class="min-h-0 flex-1"
      :min-left-px="320"
      :min-right-px="360"
    >
      <template #left>
        <section class="h-full rounded-md bg-card/60">
          <NetworkRequestPanel
            v-model="selectedTraceId"
            :requests="filteredRecords"
            :view-mode="requestViewMode"
            @select="handleRequestSelect"
          />
        </section>
      </template>

      <template #right>
        <section class="h-full rounded-md bg-card/60">
          <NetworkRequestDetail
            :record="requestStreamStore.selectedRecord"
            class="h-full"
            @rule:open="handleMatchedRuleOpen"
          />
        </section>
      </template>
    </HorizontalSplitPanel>

    <RulesAssetsDrawer
      v-model:open="rulesDrawerOpen"
      v-model:active-primary-tab="rulesDrawerPrimaryTab"
      v-model:rules-pane="rulesPane"
      v-model:selected-rule-id="selectedRuleId"
      v-model:rule-draft="ruleDraft"
      :rules="rulesStore.rules"
      :dirty="rulesStore.isDirty"
      :loading="rulesStore.loading"
      :saving="rulesStore.saving"
      @rules:create="rulesStore.createRule"
      @rules:edit="rulesStore.editRule"
      @rules:toggle-enabled="rulesStore.toggleRuleEnabled"
      @rules:bulk-toggle="rulesStore.bulkToggleSelected"
      @rules:reorder="rulesStore.reorderRules"
      @rules:delete="id => rulesStore.deleteRule(id)"
      @update:rule-draft="rulesStore.updateRuleDraft"
    />
  </div>
</template>
