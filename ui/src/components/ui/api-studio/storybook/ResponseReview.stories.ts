import type { Meta, StoryObj } from '@storybook/vue3-vite'
import ResponseReview from './ResponseReview.vue'

const meta = {
  title: 'API Studio/Response Review',
  component: ResponseReview,
  parameters: { layout: 'fullscreen' },
  args: { scenario: 'json', horizontal: false },
} satisfies Meta<typeof ResponseReview>
export default meta
type Story = StoryObj<typeof meta>
export const Playground: Story = {}
export const Json: Story = { args: { scenario: 'json' } }
export const JavaScript: Story = { args: { scenario: 'javascript' } }
export const Css: Story = { args: { scenario: 'css' } }
export const Image: Story = { args: { scenario: 'image' } }
export const Audio: Story = { args: { scenario: 'audio' } }
export const Video: Story = { args: { scenario: 'video' } }
export const Text: Story = { args: { scenario: 'text' } }
export const Html: Story = { args: { scenario: 'html' } }
export const Empty: Story = { args: { scenario: 'empty' } }
export const Binary: Story = { args: { scenario: 'binary' } }
export const LargeText: Story = { args: { scenario: 'large' } }
export const SizeLimit: Story = { args: { scenario: 'blocked' } }
export const UnknownSize: Story = { args: { scenario: 'chunked' } }
export const ReadError: Story = { args: { scenario: 'error' } }
export const Horizontal: Story = { args: { scenario: 'image', horizontal: true } }
