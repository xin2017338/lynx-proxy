<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { computed, onMounted, reactive, ref, watch } from 'vue'
import { RefreshCw } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { Spinner } from '@/components/ui/spinner'
import { cn } from '@/lib/utils'
import {
  drawerEmptyStateClass,
  drawerListItemClass,
} from '@/components/ui/rules-drawer/drawer-styles'
import type { AdbController } from './adb-controller'
import type {
  AdbDevice,
  AdbStatus,
  AndroidDevicePreview,
  DeviceRowState,
  InstallProgress,
  ProxyState,
} from './types'

const props = defineProps<{
  preview?: AndroidDevicePreview
  controller?: AdbController
  class?: HTMLAttributes['class']
}>()

const adbStatus = ref<AdbStatus>({ ready: false, source: 'none' })
const installProgress = ref<InstallProgress>({ phase: 'idle', percent: 0 })
const devices = ref<AdbDevice[]>([])
const deviceStates = reactive<Record<string, DeviceRowState>>({})
const baseAddresses = ref<string[]>([])
const localOnly = ref(false)
const port = ref(7788)
const busy = ref(false)
const initialLoading = ref(false)
const togglingSerial = ref<string | null>(null)
const error = ref<string | null>(null)

const isPreview = computed(() => props.preview != null)

const listLoading = computed(() => (
  initialLoading.value
  || (busy.value && togglingSerial.value === null && !installInFlight() && adbStatus.value.ready)
))

const headerStatusLoading = computed(() => (
  initialLoading.value || installInFlight() || (busy.value && togglingSerial.value === null)
))

function ensureRow(serial: string): DeviceRowState {
  if (!deviceStates[serial]) {
    deviceStates[serial] = { currentHttpProxy: undefined }
  }
  return deviceStates[serial]
}

function applyDeviceProxyState(state: ProxyState) {
  deviceStates[state.serial] = {
    currentHttpProxy: state.currentHttpProxy ?? null,
    proxyStateError: null,
  }
}

function proxyEnabled(serial: string): boolean {
  const value = deviceStates[serial]?.currentHttpProxy?.trim()
  return Boolean(value && value !== ':0' && value !== 'null')
}

function applyPreview(p: AndroidDevicePreview) {
  adbStatus.value = p.adbStatus
  installProgress.value = p.installProgress ?? { phase: 'idle', percent: 0 }
  devices.value = [...p.devices]
  Object.keys(deviceStates).forEach((key) => {
    delete deviceStates[key]
  })
  for (const [serial, row] of Object.entries(p.deviceStates)) {
    deviceStates[serial] = { ...row }
  }
  baseAddresses.value = [...p.baseAddresses]
  localOnly.value = p.localOnly
  port.value = p.port
  busy.value = p.busy ?? false
  initialLoading.value = p.busy ?? false
  error.value = p.error ?? null
}

watch(() => props.preview, (p) => {
  if (p) {
    applyPreview(p)
  }
}, { immediate: true, deep: true })

function installInFlight() {
  const phase = installProgress.value.phase
  return phase === 'downloading' || phase === 'extracting'
}

function canToggleProxy(device: AdbDevice): boolean {
  if (busy.value) {
    return false
  }
  if (device.state !== 'device') {
    return false
  }
  if (isPreview.value) {
    return true
  }
  return adbStatus.value.ready
}

function hostFromAddress(addr: string): string {
  const raw = addr.includes('://') ? addr.split('://')[1]! : addr
  if (raw.startsWith('[')) {
    const end = raw.indexOf(']')
    return end > 0 ? raw.slice(1, end) : raw
  }
  return raw.split(':')[0] ?? raw
}

function defaultLanHost(): string | undefined {
  for (const addr of baseAddresses.value) {
    const host = hostFromAddress(addr)
    if (host !== '127.0.0.1' && host !== 'localhost') {
      return host
    }
  }
  return undefined
}

function deviceLabel(device: AdbDevice): string {
  return device.model ?? device.serial
}

function deviceStateLabel(state: string): string {
  if (state === 'device') {
    return '已连接'
  }
  if (state === 'unauthorized') {
    return '未授权'
  }
  return state
}

function deviceStateClass(state: string): string {
  if (state === 'device') {
    return 'text-emerald-600'
  }
  if (state === 'unauthorized') {
    return 'text-amber-600 dark:text-amber-500'
  }
  return 'text-muted-foreground'
}

async function syncDeviceProxyState(serial: string) {
  const ctrl = props.controller
  if (!ctrl || isPreview.value) {
    return
  }
  try {
    const state = await ctrl.getProxyState(serial)
    applyDeviceProxyState(state)
  } catch (err) {
    deviceStates[serial] = {
      currentHttpProxy: undefined,
      proxyStateError: String(err),
    }
  }
}

async function syncAllDeviceStates() {
  await Promise.all(
    devices.value
      .filter(d => d.state === 'device')
      .map(d => syncDeviceProxyState(d.serial)),
  )
}

async function refreshAll() {
  const ctrl = props.controller
  if (!ctrl || isPreview.value) {
    return
  }
  busy.value = true
  error.value = null
  try {
    adbStatus.value = await ctrl.getStatus()
    installProgress.value = await ctrl.getInstallProgress()
    baseAddresses.value = await ctrl.getBaseAddresses()
    localOnly.value = await ctrl.getLocalOnly()
    port.value = baseAddresses.value[0]
      ? Number.parseInt(baseAddresses.value[0].split(':').pop() ?? '7788', 10)
      : 7788
    if (adbStatus.value.ready) {
      devices.value = await ctrl.listDevices()
      for (const device of devices.value) {
        ensureRow(device.serial)
      }
      await syncAllDeviceStates()
    }
  } catch (err) {
    error.value = String(err)
  } finally {
    busy.value = false
  }
}

async function onInstallAdb() {
  const ctrl = props.controller
  if (!ctrl || isPreview.value) {
    return
  }
  busy.value = true
  error.value = null
  try {
    await ctrl.install()
    const poll = window.setInterval(async () => {
      installProgress.value = await ctrl.getInstallProgress()
      if (installProgress.value.phase === 'done' || installProgress.value.phase === 'failed') {
        window.clearInterval(poll)
        if (installProgress.value.phase === 'failed') {
          error.value = installProgress.value.error ?? '安装失败'
        } else {
          await refreshAll()
        }
        busy.value = false
      }
    }, 400)
  } catch (err) {
    error.value = String(err)
    busy.value = false
  }
}

async function onRefreshDevices() {
  const ctrl = props.controller
  if (!ctrl || isPreview.value || !adbStatus.value.ready) {
    return
  }
  busy.value = true
  error.value = null
  try {
    devices.value = await ctrl.listDevices()
    for (const device of devices.value) {
      ensureRow(device.serial)
    }
    await syncAllDeviceStates()
  } catch (err) {
    error.value = String(err)
  } finally {
    busy.value = false
  }
}

async function onToggleProxy(device: AdbDevice) {
  const serial = device.serial

  if (isPreview.value) {
    const enabled = proxyEnabled(serial)
    deviceStates[serial] = {
      currentHttpProxy: enabled ? null : `${defaultLanHost() ?? 'proxy.lynx.example'}:${port.value}`,
    }
    return
  }

  const ctrl = props.controller
  if (!ctrl) {
    return
  }

  busy.value = true
  togglingSerial.value = serial
  error.value = null

  try {
    // Always decide from the device's current global setting, not cached UI state.
    const current = await ctrl.getProxyState(serial)
    applyDeviceProxyState(current)
    const wasEnabled = proxyEnabled(serial)

    if (wasEnabled) {
      applyDeviceProxyState(await ctrl.disableProxy(serial))
    } else {
      if (localOnly.value) {
        throw new Error('当前服务仅本机可访问，请关闭 --local-only 后再开启手机代理')
      }

      const host = defaultLanHost()
      if (!host) {
        throw new Error('未找到可用局域网地址，请检查网络后重试')
      }

      applyDeviceProxyState(await ctrl.enableProxy({
        serial,
        host,
        port: port.value,
      }))
    }
  } catch (err) {
    error.value = String(err)
    await syncDeviceProxyState(serial)
  } finally {
    busy.value = false
    togglingSerial.value = null
  }
}

onMounted(() => {
  if (!props.preview && props.controller) {
    initialLoading.value = true
    void refreshAll().finally(() => {
      initialLoading.value = false
    })
  }
})
</script>

<template>
  <section :class="cn('flex h-full min-h-0 flex-col overflow-hidden', props.class)">
    <div class="flex items-center justify-between gap-2 border-b border-border/60 py-2">
      <div class="flex min-w-0 items-center gap-2 text-xs">
        <Spinner
          v-if="headerStatusLoading"
          class="size-3 shrink-0 text-muted-foreground"
        />
        <span
          v-else
          class="inline-flex h-1.5 w-1.5 shrink-0 rounded-full"
          :class="adbStatus.ready ? 'bg-emerald-500' : 'bg-muted-foreground/50'"
        />
        <span class="text-muted-foreground">ADB</span>
        <span class="font-medium text-foreground">{{ adbStatus.ready ? '可用' : '不可用' }}</span>
        <Button
          v-if="!adbStatus.ready"
          variant="ghost"
          size="sm"
          class="h-6 gap-1 px-1.5 text-[11px]"
          :disabled="busy || installInFlight() || isPreview"
          @click="onInstallAdb"
        >
          <Spinner
            v-if="installInFlight()"
            class="size-3"
          />
          {{ installInFlight() ? '安装中…' : '安装' }}
        </Button>
      </div>
      <Button
        variant="ghost"
        size="sm"
        class="h-7 w-7 shrink-0 p-0"
        :disabled="busy || !adbStatus.ready || isPreview"
        aria-label="刷新设备"
        @click="onRefreshDevices"
      >
        <Spinner
          v-if="listLoading && adbStatus.ready"
          class="size-3.5"
        />
        <RefreshCw
          v-else
          class="h-3.5 w-3.5"
        />
      </Button>
    </div>

    <p
      v-if="error"
      class="mt-2 rounded-sm border border-destructive/40 bg-destructive/10 px-2 py-1 text-[11px] text-destructive"
    >
      {{ error }}
    </p>

    <div
      v-if="installInFlight()"
      class="mt-2 h-1 overflow-hidden rounded-full bg-muted"
    >
      <div
        class="h-full bg-primary transition-all"
        :style="{ width: `${installProgress.percent}%` }"
      />
    </div>

    <ul class="relative flex-1 space-y-1 overflow-auto py-2">
      <li
        v-if="listLoading"
        class="pointer-events-none absolute inset-0 z-10 flex items-center justify-center bg-background/50"
        aria-hidden="true"
      >
        <Spinner class="size-4 text-muted-foreground" />
      </li>
      <li v-if="!adbStatus.ready" :class="drawerEmptyStateClass">
        安装 ADB 后可列出已连接设备。
      </li>
      <li v-else-if="devices.length === 0" :class="drawerEmptyStateClass">
        未检测到设备，请连接 USB 并开启调试。
      </li>
      <li
        v-for="device in devices"
        v-else
        :key="device.serial"
      >
        <div :class="cn(drawerListItemClass(false), 'pl-2.5 pr-0')">
          <div class="flex items-start justify-between gap-3">
            <div class="min-w-0 flex-1">
              <p class="truncate text-xs font-semibold text-foreground">
                {{ deviceLabel(device) }}
              </p>
              <p class="mt-0.5 truncate font-mono text-[10px] text-muted-foreground">
                {{ device.serial }}
              </p>
              <p
                v-if="proxyEnabled(device.serial)"
                class="mt-0.5 truncate font-mono text-[10px] text-muted-foreground"
                :title="deviceStates[device.serial]?.currentHttpProxy ?? undefined"
              >
                代理 {{ deviceStates[device.serial]?.currentHttpProxy }}
              </p>
              <p
                v-else-if="deviceStates[device.serial]?.currentHttpProxy === undefined"
                class="mt-0.5 text-[10px] text-amber-600"
                :title="deviceStates[device.serial]?.proxyStateError ?? undefined"
              >
                代理状态未知
              </p>
            </div>
            <div
              v-if="device.state === 'device'"
              class="flex shrink-0 flex-col items-end gap-1"
            >
              <span class="text-[10px] font-medium text-emerald-600">
                {{ deviceStateLabel(device.state) }}
              </span>
              <Button
                variant="ghost"
                size="sm"
                class="h-6 min-w-[4.5rem] gap-1 px-2 text-[11px]"
                :class="proxyEnabled(device.serial) ? 'text-destructive hover:text-destructive' : ''"
                :disabled="!canToggleProxy(device) || togglingSerial === device.serial"
                @click="onToggleProxy(device)"
              >
                <Spinner
                  v-if="togglingSerial === device.serial"
                  class="size-3"
                />
                <template v-else>
                  {{ proxyEnabled(device.serial) ? '关闭代理' : '开启代理' }}
                </template>
              </Button>
            </div>
            <span
              v-else
              class="shrink-0 text-[10px] font-medium"
              :class="deviceStateClass(device.state)"
            >
              {{ deviceStateLabel(device.state) }}
            </span>
          </div>
        </div>
      </li>
    </ul>
  </section>
</template>
