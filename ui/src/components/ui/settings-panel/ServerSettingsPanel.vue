<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { computed, onMounted, ref, watch } from 'vue'
import { Copy } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { Switch } from '@/components/ui/switch'
import { cn } from '@/lib/utils'
import {
  certificateDownloadUrl,
  fetchBaseAddresses,
} from '@/lib/http/settings-api'
import { WsOp } from '@/lib/generated/ws/v1'
import type { CaptureFilter, GeneralSetting } from '@/lib/http/settings-types'
import { useGeneralSettingsStore, useWsConnectionStore } from '@/stores'
import { MAX_LOG_SIZE_MAX, MAX_LOG_SIZE_MIN } from '@/lib/http/settings-types'
import CertificateDownloadCard from './CertificateDownloadCard.vue'
import DomainFilterList from './DomainFilterList.vue'
import {
  settingsLabelClass,
  settingsMonoFieldClass,
  settingsRowGridClass,
  settingsSectionTitleClass,
  settingsValueIndentClass,
} from './settings-styles'
import type { SettingsPanelPreview } from './types'

interface ServerSettingsPanelProps {
  class?: HTMLAttributes['class']
  preview?: SettingsPanelPreview
}

const props = defineProps<ServerSettingsPanelProps>()

const wsConnection = useWsConnectionStore()
const generalSettingsStore = useGeneralSettingsStore()

const loading = ref(!props.preview)
const error = ref<string | null>(null)
const saving = ref(false)
const saveMessage = ref<string | null>(null)

const general = ref<GeneralSetting | null>(null)
const capture = ref<CaptureFilter | null>(null)
const certPath = ref('')
const baseAddresses = ref<string[]>([])
const copyMessage = ref<string | null>(null)

const isPreview = computed(() => props.preview != null)

const downloadLinks = computed(() => {
  return baseAddresses.value.map((host) => ({
    host,
    url: certificateDownloadUrl(host),
  }))
})

async function loadAll() {
  if (props.preview) {
    general.value = {
      ...props.preview.general,
      hideConnectTunnels: props.preview.general.hideConnectTunnels !== false,
    }
    capture.value = structuredClone(props.preview.capture)
    certPath.value = props.preview.certPath
    baseAddresses.value = [...props.preview.baseAddresses]
    loading.value = false
    return
  }

  loading.value = true
  error.value = null

  try {
    const [generalData, captureData, pathResult, addresses] = await Promise.all([
      wsConnection.call<GeneralSetting>(WsOp.SettingsGeneralGet),
      wsConnection.call<CaptureFilter>(WsOp.SettingsCaptureFilterGet),
      wsConnection.call<{ path: string }>(WsOp.SettingsCertificatePathGet),
      fetchBaseAddresses(),
    ])

    const hideConnect = generalData.hideConnectTunnels !== false
    general.value = {
      ...generalData,
      hideConnectTunnels: hideConnect,
    }
    generalSettingsStore.applyMaxLogSize(generalData.maxLogSize)
    generalSettingsStore.applyHideConnectTunnels(hideConnect)
    capture.value = captureData
    certPath.value = pathResult.path
    baseAddresses.value = addresses
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err)
  } finally {
    loading.value = false
  }
}

async function saveAll() {
  if (!general.value || !capture.value || isPreview.value) {
    return
  }

  saving.value = true
  saveMessage.value = null

  try {
    if (
      general.value.maxLogSize < MAX_LOG_SIZE_MIN
      || general.value.maxLogSize > MAX_LOG_SIZE_MAX
    ) {
      saveMessage.value = `日志大小需在 ${MAX_LOG_SIZE_MIN}–${MAX_LOG_SIZE_MAX} 之间`
      return
    }

    await Promise.all([
      wsConnection.call(WsOp.SettingsGeneralSet, {
        ...general.value,
        hideConnectTunnels: general.value.hideConnectTunnels !== false,
      }),
      wsConnection.call(WsOp.SettingsCaptureFilterSet, capture.value),
    ])
    generalSettingsStore.applyMaxLogSize(general.value.maxLogSize)
    generalSettingsStore.applyHideConnectTunnels(general.value.hideConnectTunnels !== false)
    saveMessage.value = '已保存'
  } catch (err) {
    saveMessage.value = err instanceof Error ? err.message : String(err)
  } finally {
    saving.value = false
  }
}

let suppressCaptureEnabledWatch = false

watch(
  () => capture.value?.enabled,
  async (enabled, previous) => {
    if (suppressCaptureEnabledWatch || isPreview.value) {
      return
    }

    // First load: don't auto-save.
    if (previous === undefined || enabled === previous) {
      return
    }

    if (!capture.value) {
      return
    }

    saving.value = true
    saveMessage.value = null

    try {
      await wsConnection.call(WsOp.SettingsCaptureFilterSet, capture.value)
      saveMessage.value = 'HTTPS 抓包设置已保存'
    } catch (err) {
      // Backend unreachable (or other failure): rollback the toggle so refresh stays consistent.
      saveMessage.value = err instanceof Error ? err.message : String(err)
      suppressCaptureEnabledWatch = true
      capture.value.enabled = previous
      suppressCaptureEnabledWatch = false
    } finally {
      saving.value = false
    }
  },
)

async function copyCertPath() {
  if (!certPath.value) {
    return
  }

  try {
    await navigator.clipboard.writeText(certPath.value)
    copyMessage.value = '已复制'
  } catch {
    copyMessage.value = '复制失败'
  }

  window.setTimeout(() => {
    copyMessage.value = null
  }, 2000)
}

onMounted(() => {
  void loadAll()
})

</script>

<template>
  <div :class="cn('flex flex-col', props.class)">
    <div
      v-if="!loading && !error && general && capture"
      class="mb-2 flex items-center justify-end gap-2"
    >
      <span
        v-if="saveMessage"
        class="tabular-nums text-xs"
        :class="saveMessage.endsWith('已保存') ? 'text-emerald-600' : 'text-destructive'"
      >
        {{ saveMessage }}
      </span>
      <Button
        size="default"
        :disabled="saving || isPreview"
        @click="saveAll"
      >
        {{ saving ? '保存中…' : '保存' }}
      </Button>
    </div>

    <div v-if="loading" class="py-8 text-center text-xs text-muted-foreground">
      加载中…
    </div>

    <div v-else-if="error" class="py-6 text-destructive">
      {{ error }}
    </div>

    <div
      v-else-if="general && capture"
      class="space-y-6"
    >
      <section class="space-y-1">
        <h2 :class="settingsSectionTitleClass">
          常规
        </h2>
        <div :class="settingsRowGridClass">
          <span :class="settingsLabelClass">日志缓存上限</span>
          <input
            v-model.number="general.maxLogSize"
            type="number"
            :min="MAX_LOG_SIZE_MIN"
            :max="MAX_LOG_SIZE_MAX"
            :class="cn(settingsMonoFieldClass, 'w-16')"
          >
        </div>
        <div :class="settingsRowGridClass">
          <span :class="settingsLabelClass">隐藏 CONNECT 隧道</span>
          <div class="justify-self-start">
            <Switch
              :checked="general.hideConnectTunnels !== false"
              :disabled="saving || isPreview"
              @update:checked="general.hideConnectTunnels = $event"
            />
          </div>
        </div>
      </section>

      <section class="space-y-1">
        <h2 :class="settingsSectionTitleClass">
          HTTPS 抓包
        </h2>
        <div :class="settingsRowGridClass">
          <span :class="settingsLabelClass">启用 HTTPS 抓包</span>
          <div class="justify-self-start">
            <Switch
              v-model:checked="capture.enabled"
              :disabled="saving || isPreview"
            />
          </div>
        </div>
        <div class="space-y-2">
          <DomainFilterList
            v-model="capture.includeDomains"
            label="包含"
          />
          <DomainFilterList
            v-model="capture.excludeDomains"
            label="排除"
          />
        </div>
      </section>

      <section class="space-y-1">
        <h2 :class="settingsSectionTitleClass">
          证书
        </h2>
        <div :class="settingsRowGridClass">
          <span :class="settingsLabelClass">根证书路径</span>
          <div class="flex min-w-0 items-center gap-1">
            <input
              type="text"
              readonly
              :class="cn(settingsMonoFieldClass, 'min-w-0 flex-1')"
              :value="certPath"
            >
            <Button
              size="icon-sm"
              variant="ghost"
              type="button"
              class="size-6 shrink-0"
              title="复制"
              @click="copyCertPath"
            >
              <Copy class="h-3 w-3" />
            </Button>
          </div>
        </div>
        <p
          v-if="copyMessage"
          :class="cn(settingsValueIndentClass, 'text-muted-foreground')"
        >
          {{ copyMessage }}
        </p>

        <div
          v-if="downloadLinks.length > 0"
          :class="cn(settingsRowGridClass, 'items-start')"
        >
          <span :class="cn(settingsLabelClass, 'pt-1')">安装证书</span>
          <div class="space-y-2">
            <CertificateDownloadCard
              v-for="item in downloadLinks"
              :key="item.host"
              :host="item.host"
              :url="item.url"
            />
          </div>
        </div>

        <p
          :class="cn(settingsValueIndentClass, 'text-muted-foreground leading-relaxed')"
        >
          安装为受信任根证书后即可解密 HTTPS。移动端需在 Wi‑Fi 代理中填写上方局域网地址（勿填
          127.0.0.1），并在本机安装用户 CA；若终端出现 CertificateUnknown，说明手机尚未信任该证书。
        </p>
      </section>
    </div>
  </div>
</template>
