import type { Meta, StoryObj } from '@storybook/vue3-vite'
import { ref } from 'vue'
import RulesAssetsDrawer from '@/components/ui/rules-drawer/RulesAssetsDrawer.vue'
import type { RuleWorkbenchRuleItem } from '@/components/ui/rule-workbench'
import AndroidDevicePanel from './AndroidDevicePanel.vue'
import {
  PREVIEW_ADB_DOWNLOADING,
  PREVIEW_ADB_INSTALL_FAILED,
  PREVIEW_ADB_NOT_INSTALLED,
  PREVIEW_DEVICES_LISTED,
  PREVIEW_LAN_BLOCKED,
  PREVIEW_PROXY_ENABLED,
  PREVIEW_READY_NO_DEVICES,
} from './fixtures'

const meta = {
  title: 'Device/AndroidDevicePanel',
  component: AndroidDevicePanel,
  parameters: {
    layout: 'fullscreen',
  },
} satisfies Meta<typeof AndroidDevicePanel>

export default meta

type Story = StoryObj<typeof meta>

function panelStory(preview: typeof PREVIEW_READY_NO_DEVICES) {
  return {
    render: () => ({
      components: { AndroidDevicePanel },
      setup() {
        return { preview }
      },
      template: `
        <div class="h-[520px] w-[360px] border border-border bg-background px-3 pb-3 pt-2">
          <AndroidDevicePanel :preview="preview" class="h-full" />
        </div>
      `,
    }),
  }
}

export const Default_Ready_NoDevices: Story = panelStory(PREVIEW_READY_NO_DEVICES)
export const Devices_Listed: Story = panelStory(PREVIEW_DEVICES_LISTED)
export const Adb_NotInstalled: Story = panelStory(PREVIEW_ADB_NOT_INSTALLED)
export const Adb_Downloading: Story = panelStory(PREVIEW_ADB_DOWNLOADING)
export const Adb_InstallFailed: Story = panelStory(PREVIEW_ADB_INSTALL_FAILED)
export const Proxy_Enabled: Story = panelStory(PREVIEW_PROXY_ENABLED)
export const LanBlocked_LocalOnly: Story = panelStory(PREVIEW_LAN_BLOCKED)

export const Loading: Story = {
  render: () => ({
    components: { AndroidDevicePanel },
    setup() {
      return {
        preview: {
          ...PREVIEW_DEVICES_LISTED,
          busy: true,
        },
      }
    },
    template: `
      <div class="h-[520px] w-[360px] border border-border bg-background px-3 pb-3 pt-2">
        <AndroidDevicePanel :preview="preview" class="h-full" />
      </div>
    `,
  }),
}

const SAMPLE_RULES: RuleWorkbenchRuleItem[] = [
  {
    id: 'rule-001',
    name: 'Sample Rule',
    enabled: true,
    priority: 50,
    state: 'valid',
    summary: 'Story fixture',
  },
]

export const InRulesDrawer: Story = {
  render: () => ({
    components: { RulesAssetsDrawer },
    setup() {
      const open = ref(true)
      const activePrimaryTab = ref<'device'>('device')
      return {
        open,
        activePrimaryTab,
        rules: SAMPLE_RULES,
        androidPreview: PREVIEW_DEVICES_LISTED,
      }
    },
    template: `
      <RulesAssetsDrawer
        v-model:open="open"
        v-model:active-primary-tab="activePrimaryTab"
        :rules="rules"
        :android-preview="androidPreview"
      />
    `,
  }),
}
