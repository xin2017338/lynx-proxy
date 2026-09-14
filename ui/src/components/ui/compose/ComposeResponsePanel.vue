<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import type { ComposeResponse } from './types'
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { AlertCircle, Copy, Download, Minus, Plus } from '@lucide/vue'
import CodeMirrorSurface from '@/components/ui/content-workbench/CodeMirrorSurface.vue'
import type { WorkbenchLanguage } from '@/components/ui/content-workbench/utils'
import { cn } from '@/lib/utils'
import { formatByteSize } from '@/lib/http/body-display-limits'
import { RESPONSE_RENDER_LIMIT, RESPONSE_PREVIEW_CHARS, responseBytes, responseHex, responseKind, responseMime, responseTabs } from './lib/response-view'
import { apiStudioGhostButtonClass, apiStudioIconButtonClass, apiStudioUnderlineTabClass, apiStudioUnderlineTabActiveClass, apiStudioUnderlineTabIdleClass } from '@/components/ui/api-studio/api-studio-styles'

const props = defineProps<{ response?: ComposeResponse | null; loading?: boolean; error?: string | null; hideStatusBar?: boolean; class?: HTMLAttributes['class'] }>()
const active = ref('headers')
const zoom = ref(100)
const copyLabel = ref('复制')
const mediaError = ref(false)
const operationError = ref('')
const mediaUrl = ref('')
let copyTimer: ReturnType<typeof setTimeout> | undefined
const kind = computed(() => responseKind(props.response))
const mime = computed(() => responseMime(props.response))
const tabs = computed(() => responseTabs(props.response))
const labels: Record<string, string> = { formatted: '格式化', raw: '原始内容', headers: 'Headers', preview: '预览', file: '文件', notice: '响应提示' }
const large = computed(() => (props.response?.size ?? 0) > RESPONSE_RENDER_LIMIT || (props.response?.body.length ?? 0) > RESPONSE_RENDER_LIMIT)
const binary = computed(() => props.response?.bodyEncoding === 'base64')
const media = computed(() => ['image', 'audio', 'video'].includes(kind.value))
const error = computed(() => props.response?.status ? null : props.error ?? props.response?.errorMessage)
const raw = computed(() => !props.response ? '' : binary.value ? responseHex(props.response) : props.response.body.slice(0, large.value ? RESPONSE_PREVIEW_CHARS : RESPONSE_RENDER_LIMIT))
const formatted = computed(() => {
  if (active.value !== 'formatted' || large.value || !props.response) return ''
  if (kind.value === 'json') {
    try {
      const result = JSON.stringify(JSON.parse(props.response.body), null, 2)
      return result.length <= RESPONSE_RENDER_LIMIT ? result : props.response.body
    } catch { return props.response.body }
  }
  return props.response.body
})
const language = computed(() => kind.value as WorkbenchLanguage)
function revokeMedia() { if (mediaUrl.value) URL.revokeObjectURL(mediaUrl.value); mediaUrl.value = '' }
watch(() => props.response, () => {
  active.value = tabs.value[0] ?? 'headers'; zoom.value = 100; mediaError.value = false; operationError.value = ''; copyLabel.value = '复制'
  revokeMedia()
}, { immediate: true })
watch([active, () => props.response], () => {
  if (active.value !== 'preview') { revokeMedia(); return }
  if (!props.response || mediaUrl.value) return
  try { mediaUrl.value = URL.createObjectURL(new Blob([new Uint8Array(responseBytes(props.response))], { type: mime.value })) }
  catch { mediaError.value = true }
}, { immediate: true })
async function copy() {
  if (!props.response) return
  try { await navigator.clipboard.writeText(active.value === 'formatted' ? formatted.value : raw.value); copyLabel.value = '已复制' }
  catch { copyLabel.value = '复制失败' }
  clearTimeout(copyTimer); copyTimer = setTimeout(() => { copyLabel.value = '复制' }, 1600)
}
function download() {
  if (!props.response || props.response.bodyState) return
  try {
    const url = URL.createObjectURL(new Blob([new Uint8Array(responseBytes(props.response))], { type: mime.value || 'application/octet-stream' }))
    const extensions: Record<string, string> = { 'application/json': 'json', 'text/html': 'html', 'text/css': 'css', 'application/javascript': 'js', 'text/javascript': 'js', 'application/xml': 'xml', 'text/plain': 'txt', 'image/png': 'png', 'image/jpeg': 'jpg', 'image/svg+xml': 'svg', 'image/gif': 'gif', 'image/webp': 'webp', 'audio/mpeg': 'mp3', 'audio/wav': 'wav', 'video/mp4': 'mp4', 'video/webm': 'webm' }
    const link = document.createElement('a'); link.href = url; link.download = `response.${extensions[mime.value] ?? 'bin'}`; link.click()
    setTimeout(() => URL.revokeObjectURL(url), 1000)
  } catch { operationError.value = '下载失败，响应内容无法解码。' }
}
onBeforeUnmount(() => { revokeMedia(); clearTimeout(copyTimer) })
</script>

<template>
  <section :class="cn('flex h-full min-h-0 flex-col overflow-hidden px-2 pb-2', props.class)">
    <nav class="flex shrink-0 flex-wrap items-center justify-between gap-1 border-b border-border/60">
      <div class="flex overflow-x-auto"><button v-for="tab in tabs" :key="tab" type="button" :class="[apiStudioUnderlineTabClass, active === tab ? apiStudioUnderlineTabActiveClass : apiStudioUnderlineTabIdleClass]" @click="active = tab">{{ labels[tab] }}</button></div>
      <div class="ml-auto min-w-0 text-[10px] text-muted-foreground">
        <slot v-if="hideStatusBar" name="tab-trailing" />
        <span v-else-if="response && !loading" class="flex gap-2"><span :class="response.status >= 400 ? 'text-destructive' : 'text-foreground'">{{ response.status }} {{ response.statusText }}</span><span>{{ response.responseTime }} ms</span><span>{{ response.sizeKnown === false ? '≥ ' : '' }}{{ formatByteSize(response.size) }}</span></span>
      </div>
    </nav>
    <div v-if="loading" class="flex flex-1 items-center justify-center text-xs text-muted-foreground">发送中…</div>
    <div v-else-if="error" class="flex flex-1 items-center justify-center p-4 text-xs text-destructive">{{ error }}</div>
    <div v-else-if="!response" class="flex flex-1 items-center justify-center text-xs text-muted-foreground">暂无数据</div>
    <div v-else class="flex min-h-0 flex-1 flex-col overflow-auto rounded-sm bg-muted/15">
      <div v-if="active === 'headers'" class="p-2">
        <div v-for="(value, key) in response.headers" :key="key" class="grid grid-cols-[minmax(7rem,30%)_1fr] gap-3 py-1 font-mono text-[11px]"><span class="break-all text-muted-foreground">{{ key }}</span><span class="break-all">{{ value }}</span></div>
        <p v-if="!Object.keys(response.headers).length" class="text-xs text-muted-foreground">无响应 Headers</p>
      </div>
      <div v-else-if="active === 'notice'" class="flex flex-1 items-center justify-center p-6">
        <div class="max-w-md space-y-3 text-xs"><AlertCircle class="size-5 text-muted-foreground" /><p>{{ response.bodyMessage || response.errorMessage || '未获得完整响应正文。' }}</p><p class="text-muted-foreground">{{ response.sizeKnown === false ? '已接收' : '响应大小' }} {{ formatByteSize(response.size) }}{{ response.sizeKnown === false ? '，总大小未知。' : '。' }}状态码与 Headers 已保留。</p><a v-if="response.bodyState === 'blocked'" href="/settings#api-studio-settings" :class="apiStudioGhostButtonClass">调整大小限制</a></div>
      </div>
      <template v-else>
        <div class="flex shrink-0 items-center gap-2 border-b border-border/40 px-2 py-1 text-[10px] text-muted-foreground">
          <span>{{ mime || '未知类型' }}</span><span v-if="binary && active === 'raw'">· Hex</span>
          <div class="ml-auto flex items-center gap-1">
            <template v-if="kind === 'image' && active === 'preview'"><button :class="apiStudioIconButtonClass" aria-label="缩小图片" :disabled="zoom <= 25" @click="zoom -= 25"><Minus class="size-3.5" /></button><span class="w-10 text-center tabular-nums">{{ zoom }}%</span><button :class="apiStudioIconButtonClass" aria-label="放大图片" :disabled="zoom >= 300" @click="zoom += 25"><Plus class="size-3.5" /></button></template>
            <button v-if="!media && !binary" :class="apiStudioGhostButtonClass" @click="copy"><Copy class="inline size-3" /> {{ large && copyLabel === '复制' ? '复制预览' : copyLabel }}</button>
            <button :class="apiStudioGhostButtonClass" @click="download"><Download class="inline size-3" /> 下载</button>
          </div>
        </div>
        <p v-if="operationError" class="p-3 text-[11px] text-destructive">{{ operationError }}</p>
        <div v-if="active === 'preview'" class="flex min-h-0 flex-1 flex-col overflow-auto p-4">
          <p v-if="mediaError" class="m-auto text-xs text-muted-foreground">无法预览此媒体格式，请下载查看。</p>
          <img v-else-if="kind === 'image'" :src="mediaUrl" alt="响应图片" :style="{ width: `${zoom}%`, maxWidth: zoom === 100 ? '100%' : 'none' }" class="m-auto shrink-0 object-contain" @error="mediaError = true">
          <audio v-else-if="kind === 'audio'" :src="mediaUrl" controls preload="metadata" class="m-auto w-full max-w-md" @error="mediaError = true" />
          <video v-else :src="mediaUrl" controls preload="metadata" class="m-auto max-h-full max-w-full" @error="mediaError = true" />
        </div>
        <div v-else-if="active === 'file'" class="flex flex-1 flex-col items-center justify-center gap-2 p-4 text-xs"><p>二进制文件 · {{ formatByteSize(response.size) }}</p><p class="text-muted-foreground">此类型不支持预览，可下载查看或切换原始内容。</p></div>
        <CodeMirrorSurface v-else-if="active === 'formatted'" :content="formatted" :language="language" compact soft-wrap />
        <div v-else class="p-3 text-[11px]">
          <p v-if="large || binary" class="mb-3 leading-relaxed text-muted-foreground">{{ binary ? '下方展示原始字节预览。' : `正文 ${formatByteSize(response.size)}，已关闭格式化与高亮。下方仅展示部分内容。` }}可下载完整正文。</p>
          <pre class="whitespace-pre-wrap break-all font-mono text-[11px]">{{ raw }}</pre>
          <p v-if="(large && response.body.length > RESPONSE_PREVIEW_CHARS) || (binary && response.body.length > 4096)" class="mt-2 text-muted-foreground">…（剩余内容未展示）</p>
        </div>
      </template>
    </div>
  </section>
</template>
