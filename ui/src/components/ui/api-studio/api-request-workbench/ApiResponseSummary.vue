<script setup lang="ts">
import type { ComposeResponse } from '../types'
import { computed } from 'vue'
import { Loader2 } from '@lucide/vue'
import { cn } from '@/lib/utils'
import { apiStudioHintTextClass } from '../api-studio-styles'

const props = defineProps<{
  response?: ComposeResponse | null
  loading?: boolean
  error?: string | null
}>()

function bytes(value: number): string {
  if (value < 1024) return `${value} B`
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KB`
  return `${(value / (1024 * 1024)).toFixed(2)} MB`
}

function statusTone(status: number): string {
  if (status >= 200 && status < 300) return 'text-emerald-600 dark:text-emerald-400'
  if (status >= 300 && status < 400) return 'text-amber-600 dark:text-amber-400'
  if (status >= 400) return 'text-destructive'
  return 'text-foreground'
}

const errorText = computed(() => props.error ?? null)

const responseSummary = computed(() => {
  if (props.loading || props.error || !props.response) return null
  const { status, statusText, responseTime, size } = props.response
  return {
    status,
    statusText,
    responseTime,
    sizeLabel: `${props.response.sizeKnown === false ? '≥ ' : ''}${bytes(size)}`,
  }
})
</script>

<template>
  <div
    class="flex min-w-0 items-center justify-end gap-2 truncate text-[10px]"
    :title="props.error ?? undefined"
  >
    <template v-if="props.loading">
      <Loader2 class="size-3.5 shrink-0 animate-spin text-muted-foreground" aria-hidden="true" />
      <span :class="apiStudioHintTextClass">发送中…</span>
    </template>

    <span
      v-else-if="errorText"
      class="truncate text-destructive"
    >
      {{ errorText }}
    </span>

    <template v-else-if="responseSummary">
      <span :class="cn('shrink-0 font-mono font-semibold', statusTone(responseSummary.status))">
        {{ responseSummary.status }}
      </span>
      <span class="truncate text-muted-foreground">
        {{ responseSummary.statusText }}
      </span>
      <span class="shrink-0 text-muted-foreground">
        {{ responseSummary.responseTime }}ms
      </span>
      <span class="shrink-0 text-muted-foreground">
        {{ responseSummary.sizeLabel }}
      </span>
    </template>
  </div>
</template>
