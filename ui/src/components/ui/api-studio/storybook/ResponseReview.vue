<script setup lang="ts">
// Storybook-only interaction prototype. No production request or settings integration.
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import videoUrl from './preview.webm?url'
import { Copy, Download, Settings2, Columns2, Minus, Plus, AlertCircle } from '@lucide/vue'
import { ContentWorkbench } from '@/components/ui/content-workbench'
import ApiRequestEditor from '../api-request-workbench/ApiRequestEditor.vue'
import ApiRequestBar from '../api-request-workbench/ApiRequestBar.vue'
import { createEmptyApiDraft } from '../lib/empty-api-draft'
import type { ApiRequestTab } from '../types'
import { apiStudioFieldClass, apiStudioGhostButtonClass, apiStudioIconButtonClass, apiStudioPrimaryButtonClass, apiStudioUnderlineTabClass, apiStudioUnderlineTabActiveClass, apiStudioUnderlineTabIdleClass } from '../api-studio-styles'

const props = withDefaults(defineProps<{ scenario?: string; horizontal?: boolean }>(), { scenario: 'json' })
const scenarios = ['json', 'image', 'audio', 'video', 'text', 'html', 'javascript', 'css', 'empty', 'binary', 'large', 'blocked', 'chunked', 'error']
const labels: Record<string, string> = { json: 'JSON', image: '图片', audio: '音频', video: '视频', text: '文本', html: 'HTML', javascript: 'JavaScript', css: 'CSS', empty: '空响应', binary: '二进制', large: '大文本', blocked: '超限 · 已知大小', chunked: '超限 · 未知大小', error: '读取失败' }
const scenario = ref(props.scenario)
const horizontal = ref(props.horizontal)
const settingsOpen = ref(false)
const limit = ref(50)
const limitInput = ref(50)
const settingsError = ref('')
const requestTab = ref<ApiRequestTab>('params')
const draft = ref(createEmptyApiDraft({ url: 'https://api.example.com/v1/response', name: 'Response preview' }))
const active = ref('formatted')
const zoom = ref(100)
const mediaError = ref(false)
watch(() => props.scenario, value => scenario.value = value)
watch(() => props.horizontal, value => horizontal.value = value)
const blocked = computed(() => ['blocked', 'chunked'].includes(scenario.value))
const media = computed(() => ['image', 'audio', 'video'].includes(scenario.value))
const mime = computed(() => ({ json: 'application/json', image: 'image/svg+xml', audio: 'audio/wav', video: 'video/webm', text: 'text/plain', html: 'text/html', javascript: 'application/javascript', css: 'text/css', empty: '', binary: 'application/octet-stream', large: 'application/json', blocked: 'application/octet-stream', chunked: 'application/octet-stream', error: 'application/json' })[scenario.value] ?? 'text/plain')
const tabs = computed(() => blocked.value || scenario.value === 'error' ? ['notice', 'headers'] : scenario.value === 'empty' ? ['headers'] : media.value ? ['preview', 'raw', 'headers'] : scenario.value === 'binary' ? ['file', 'raw', 'headers'] : ['json', 'html', 'javascript', 'css'].includes(scenario.value) ? ['formatted', 'raw', 'headers'] : ['raw', 'headers'])
const tabLabels: Record<string, string> = { formatted: '格式化', raw: '原始内容', preview: '预览', headers: 'Headers', notice: '响应提示', file: '文件' }
watch(scenario, () => { active.value = tabs.value[0]!; zoom.value = 100; mediaError.value = false }, { immediate: true })
const sample = '{"ok":true,"data":{"id":"req_2048","name":"Lynx API Studio","tags":["preview","response"],"duration":128}}'
const content = computed(() => scenario.value === 'javascript' ? 'export async function fetchSession(userId) {\n  const response = await fetch(`/api/sessions/${userId}`)\n  if (!response.ok) throw new Error("Request failed")\n  return response.json()\n}\n\n// Load the current session\nfetchSession(42).then(console.log)\n' : scenario.value === 'css' ? ':root {\n  --accent: #00a896;\n  --radius: 4px;\n}\n\n.response-panel {\n  display: flex;\n  flex-direction: column;\n  padding: 12px;\n  border-radius: var(--radius);\n}\n\n.response-panel:hover {\n  border-color: var(--accent);\n}\n' : scenario.value === 'html' ? '<!doctype html>\n<html>\n  <body><h1>Hello, Lynx</h1></body>\n</html>' : scenario.value === 'text' ? 'Request completed successfully.\nTrace: req_2048\nRegion: ap-east-1' : sample)
const size = computed(() => scenario.value === 'blocked' ? '86.4 MB' : scenario.value === 'chunked' ? `> ${limit.value} MB` : scenario.value === 'large' ? '8.2 MB' : scenario.value === 'empty' ? '0 B' : media.value ? '24.0 KB' : '128 B')
const headers = computed(() => ({ 'content-type': mime.value || '(无)', ...(scenario.value === 'chunked' ? { 'transfer-encoding': 'chunked' } : { 'content-length': scenario.value === 'blocked' ? '90596966' : '128' }), 'x-request-id': 'req_2048', 'cache-control': 'no-cache' }))
const imageUrl = 'data:image/svg+xml,' + encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" width="720" height="360" viewBox="0 0 720 360"><rect width="720" height="360" fill="#17232d"/><circle cx="560" cy="100" r="150" fill="#254f50"/><path d="M0 340L220 130 380 300 510 200 720 360H0" fill="#6baaa2"/><text x="40" y="70" fill="#ecf5f4" font-size="28" font-family="sans-serif">Lynx · Response preview</text></svg>')
// Deterministic local WAV: one second of quiet 440 Hz tone.
const wav = new Uint8Array(8044)
const view = new DataView(wav.buffer)
for (const [offset, text] of [[0, 'RIFF'], [8, 'WAVE'], [12, 'fmt '], [36, 'data']] as const) for (let i = 0; i < text.length; i++) wav[offset + i] = text.charCodeAt(i)
view.setUint32(4, 8036, true); view.setUint32(16, 16, true); view.setUint16(20, 1, true); view.setUint16(22, 1, true); view.setUint32(24, 8000, true); view.setUint32(28, 8000, true); view.setUint16(32, 1, true); view.setUint16(34, 8, true); view.setUint32(40, 8000, true)
for (let i = 44; i < wav.length; i++) wav[i] = 128 + Math.round(12 * Math.sin((i - 44) * Math.PI * 880 / 8000))
const audioUrl = 'data:audio/wav;base64,' + btoa(String.fromCharCode(...wav))
function saveSettings() {
  if (!Number.isFinite(limitInput.value) || limitInput.value <= 0) { settingsError.value = '请输入大于 0 的响应大小'; return }
  limit.value = limitInput.value; settingsError.value = ''; settingsOpen.value = false
}
const copyLabel = ref('复制')
let copyTimer: ReturnType<typeof setTimeout> | undefined
async function copyContent() {
  try {
    const text = active.value === 'formatted' && scenario.value === 'json'
      ? JSON.stringify(JSON.parse(content.value), null, 2) : content.value
    await navigator.clipboard.writeText(text)
    copyLabel.value = '已复制'
  } catch { copyLabel.value = '复制失败' }
  clearTimeout(copyTimer)
  copyTimer = setTimeout(() => { copyLabel.value = '复制' }, 1600)
}
onBeforeUnmount(() => clearTimeout(copyTimer))
function download() {
  const a = document.createElement('a')
  a.href = scenario.value === 'image' ? imageUrl : scenario.value === 'audio' ? audioUrl : scenario.value === 'video' ? videoUrl : 'data:application/octet-stream;base64,' + btoa(content.value)
  a.download = scenario.value === 'image' ? 'response.svg' : scenario.value === 'audio' ? 'response.wav' : scenario.value === 'video' ? 'response.webm' : scenario.value === 'javascript' ? 'response.js' : scenario.value === 'css' ? 'response.css' : scenario.value === 'json' ? 'response.json' : scenario.value === 'html' ? 'response.html' : 'response.txt'
  a.click()
}
</script>

<template>
  <div>
    <div class="flex h-screen min-h-[600px] flex-col bg-background text-foreground">
      <div class="flex flex-wrap items-center gap-2 border-b border-border px-3 py-2">
        <span class="text-xs font-semibold">API Studio</span>
        <span class="text-[10px] text-muted-foreground">交互评审</span>
        <select v-model="scenario" aria-label="响应场景" :class="apiStudioFieldClass"><option v-for="s in scenarios" :key="s" :value="s">{{ labels[s] }}</option></select>
        <div class="ml-auto flex gap-1">
          <button :class="apiStudioIconButtonClass" aria-label="切换布局" @click="horizontal = !horizontal"><Columns2 class="size-3.5" /></button>
          <button :class="apiStudioIconButtonClass" aria-label="API Studio 设置" @click="settingsOpen = !settingsOpen; limitInput = limit"><Settings2 class="size-3.5" /></button>
        </div>
      </div>
      <div v-if="settingsOpen" class="space-y-2 border-b border-border bg-muted/15 px-4 py-3">
        <p class="text-xs font-medium">API Studio 设置</p>
        <div class="flex items-center gap-3"><label for="response-limit" class="text-xs">最大响应大小（MB）</label><input id="response-limit" v-model.number="limitInput" type="number" min="1" :class="[apiStudioFieldClass, 'w-24']"><button :class="apiStudioPrimaryButtonClass" @click="saveSettings">保存</button><button :class="apiStudioGhostButtonClass" @click="settingsOpen = false">取消</button></div>
        <p class="text-[10px] text-muted-foreground">默认 50 MB，对后续请求生效。超过上限时停止接收正文。</p>
        <p v-if="settingsError" class="text-xs text-destructive">{{ settingsError }}</p>
      </div>
      <ApiRequestBar :draft="draft" compact @update:url="draft.url = $event" @update:method="draft.method = $event" />
      <div :class="['flex min-h-0 flex-1', horizontal ? 'flex-row' : 'flex-col']">
        <div :class="horizontal ? 'w-[45%] border-r border-border p-2' : 'h-[190px] shrink-0 border-b border-border p-2'"><ApiRequestEditor v-model:draft="draft" :active-tab="requestTab" :layout="horizontal ? 'horizontal' : 'vertical'" :can-copy-curl="false" compact @toggle-layout="horizontal = !horizontal" @update:active-tab="requestTab = $event" /></div>
        <section class="flex min-h-0 min-w-0 flex-1 flex-col px-2 pb-2">
          <nav class="flex flex-wrap items-center justify-between border-b border-border/60">
            <div class="flex"><button v-for="tab in tabs" :key="tab" :class="[apiStudioUnderlineTabClass, active === tab ? apiStudioUnderlineTabActiveClass : apiStudioUnderlineTabIdleClass]" @click="active = tab">{{ tabLabels[tab] }}</button></div>
            <div class="flex gap-2 px-2 text-[10px] text-muted-foreground"><span class="font-mono text-emerald-600 dark:text-emerald-400">{{ scenario === 'empty' ? '204 No Content' : '200 OK' }}</span><span>128 ms</span><span>{{ size }}</span></div>
          </nav>
          <div class="flex min-h-0 flex-1 flex-col overflow-auto bg-muted/15">
            <div v-if="active === 'headers'" class="p-2"><div v-for="(value, key) in headers" :key="key" class="grid grid-cols-[minmax(8rem,30%)_1fr] gap-3 py-1 font-mono text-[11px]"><span class="text-muted-foreground">{{ key }}</span><span class="break-all">{{ value }}</span></div></div>
            <div v-else-if="active === 'notice'" class="flex flex-1 items-center justify-center p-6"><div class="max-w-md space-y-3"><AlertCircle class="size-5 text-muted-foreground" /><p class="text-xs font-medium">{{ blocked ? `响应超过 ${limit} MB，已停止接收正文` : '响应正文读取失败' }}</p><p class="text-xs leading-relaxed text-muted-foreground">{{ scenario === 'blocked' ? '服务器声明响应大小为 86.4 MB。状态码与 Headers 已保留。' : scenario === 'chunked' ? `已接收超过 ${limit} MB，总大小未知。状态码与 Headers 已保留。` : '连接在接收过程中断开，未获得完整正文。可查看 Headers 或重新发送请求。' }}</p><button v-if="blocked" :class="apiStudioGhostButtonClass" @click="settingsOpen = true; limitInput = limit">调整大小限制</button></div></div>
            <template v-else>
              <div class="flex items-center gap-2 border-b border-border/40 px-2 py-1 text-[10px] text-muted-foreground"><span>{{ mime }}</span><span v-if="active === 'raw' && (media || scenario === 'binary')">· Hex</span><div class="ml-auto flex items-center gap-1"><div v-if="scenario === 'image' && active === 'preview'" class="flex items-center gap-1"><button :class="apiStudioIconButtonClass" aria-label="缩小图片" :disabled="zoom <= 25" @click="zoom = Math.max(25, zoom - 25)"><Minus class="size-3.5" /></button><span class="w-10 text-center tabular-nums" aria-live="polite">{{ zoom }}%</span><button :class="apiStudioIconButtonClass" aria-label="放大图片" :disabled="zoom >= 300" @click="zoom = Math.min(300, zoom + 25)"><Plus class="size-3.5" /></button></div><button v-if="!media && scenario !== 'binary' && scenario !== 'large'" :class="apiStudioGhostButtonClass" @click="copyContent"><Copy class="inline size-3" /> {{ copyLabel }}</button><button v-if="scenario !== 'large'" :class="apiStudioGhostButtonClass" @click="download"><Download class="inline size-3" /> 下载</button></div></div>
              <div v-if="active === 'preview'" class="flex min-h-0 flex-1 items-center justify-center overflow-auto p-4">
                <img v-if="scenario === 'image'" :src="imageUrl" alt="Lynx 响应预览示例" :style="{ width: zoom + '%', maxWidth: zoom === 100 ? '720px' : 'none' }" class="shrink-0">
                <audio v-else-if="scenario === 'audio'" :src="audioUrl" controls preload="metadata" class="w-full max-w-md" />
                <div v-else class="w-full max-w-xl"><video :src="videoUrl" controls preload="metadata" :poster="imageUrl" class="w-full rounded-sm" @error="mediaError = true" /><p class="mt-2 text-[10px] text-muted-foreground">{{ mediaError ? '视频加载失败，请下载查看。' : '本地视频样例 · 2 秒' }}</p></div>
              </div>
              <div v-else-if="active === 'file'" class="flex flex-1 flex-col items-center justify-center gap-2 text-xs"><p>response.bin</p><p class="text-muted-foreground">此类型不支持预览，可下载查看或切换原始内容。</p></div>
              <pre v-else-if="media || scenario === 'binary'" class="p-3 font-mono text-[11px] text-muted-foreground">00000000  4c 59 4e 58 00 01 02 03  LYNX....

原始字节展示样式示例</pre>
              <ContentWorkbench v-else-if="active === 'formatted'" :type="scenario === 'html' ? 'html-source' : scenario === 'javascript' ? 'javascript-source' : scenario === 'css' ? 'css-source' : 'json'" :content="content" :show-copy="false" class="[&>header]:hidden" frameless />
              <div v-else class="p-3 text-[11px]">
                <p v-if="scenario === 'large'" class="mb-3 leading-relaxed text-muted-foreground">正文 8.2 MB，已关闭格式化与高亮。下方仅展示部分内容。完整下载将在集成后提供</p>
                <pre class="whitespace-pre-wrap break-all font-mono text-[11px]">{{ content }}{{ scenario === 'large' ? '\n…（剩余内容未展示）' : '' }}</pre>
              </div>
            </template>
          </div>
        </section>
      </div>
    </div>
  </div>
</template>
