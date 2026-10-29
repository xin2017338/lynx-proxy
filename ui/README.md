# UI Structure Guide

This UI project uses a layered structure so component development, state management, and app usage remain clear as the codebase grows.

## Directory Layout

```text
src/
	components/
		index.ts
		ui/
			index.ts
			primitives/
				index.ts
				button/
					index.ts
	stores/
		index.ts
		counter.ts
		modules/
			index.ts
			counter.store.ts
	views/
	router/
	stories/
		Configure.mdx
	lib/
```

## Layer Responsibilities

- `components/ui/primitives`: base reusable UI atoms (button, input, badge, icon button).
- `components/ui`: UI composition entry for the UI layer.
- `components/index.ts`: top-level export entry used by app code and stories.
- `stores/modules`: domain/global stores grouped by module.
- `stores/index.ts`: top-level store exports.
- `views`: route pages that consume components and stores.
- `stories`: docs-only entries (MDX pages).

## Import Convention

- App and stories should import components from `@/components`.
- App and views should import stores from `@/stores`.
- Keep old file-level imports only as compatibility shims during migration.

Examples:

```ts
import { Button } from '@/components'
import { useCounterStore } from '@/stores'
```

## Component Development Flow

1. Create a component in `components/ui/primitives/<component-name>/`.
2. Export it from local `index.ts` and upper-level barrel files.
3. Add or update a colocated story in the same component folder for visual verification.
4. Use the component in `views` or feature modules via `@/components`.

Example colocated story path:

```text
src/components/ui/primitives/button/Button.stories.ts
```

## Store Development Flow

1. Add a store in `stores/modules/<name>.store.ts`.
2. Export it from `stores/modules/index.ts`.
3. Re-export from `stores/index.ts`.
4. Consume it from views/components via `@/stores`.

## Theme Token System

Theme foundation is centralized in `src/style.css` and applied app-wide with `src/composables/useTheme.ts`.

### Token Groups

- Color: `--theme-bg`, `--theme-surface`, `--theme-surface-muted`, `--theme-border`, `--theme-text`, `--theme-text-muted`, `--theme-primary`, `--theme-success`, `--theme-info`
- Radius: `--radius-sm`, `--radius-md`, `--radius-lg`, `--radius-xl`
- Spacing: `--space-2xs`, `--space-xs`, `--space-sm`, `--space-md`, `--space-lg`, `--space-xl`
- Shadow: `--shadow-sm`, `--shadow-md`
- Typography: `--font-sans`, `--font-mono`

### Theme Modes

- `light`: explicit light theme (via `html.light`)
- `dark`: explicit dark theme (via `html.dark`)
- `system`: follows OS preference and updates when system preference changes

### Runtime API

Use `useTheme` from `src/composables/useTheme.ts`:

```ts
const { themeMode, setTheme, cycleTheme, resolvedTheme } = useTheme()
```

Initialize once near app root:

```ts
initializeTheme()
```

### Styling Rule

- Prefer semantic token utilities in templates, for example `bg-(--theme-surface)` and `text-(--theme-text)`.
- Do not hardcode brand colors, spacing, or radius inside components.
- Keep shadcn semantic tokens (`--primary`, `--background`, etc.) mapped from theme tokens in `src/style.css`.

## README screenshots

Root README images are captured from this UI against a live Lynx proxy.

1. `task dev` — proxy on `:7788` and Vite on `:5173`
2. `task readme-demo` — httpbin + lynx-mock traffic, WebSocket, demo rules
3. `task readme-screenshots` or `npm run screenshots:readme` — writes `../images/*.png`

Uses system **Google Chrome** by default (`channel: 'chrome'`). Override with `PLAYWRIGHT_CHANNEL` or `PLAYWRIGHT_EXECUTABLE_PATH` if needed.

Demo rules live in `../scripts/fixtures/demo-rules.json`.

## DSL WASM sync

- UI scripts now run `npm run dsl:ensure` before `dev`, `build`, `test:unit`, and Storybook commands.
- `dsl:ensure` checks `../crates/lynx-dsl` sources against `src/wasm/lynx-dsl/*` timestamps.
- If Rust DSL sources are newer (or artifacts are missing), it automatically runs `npm run dsl:build`.
