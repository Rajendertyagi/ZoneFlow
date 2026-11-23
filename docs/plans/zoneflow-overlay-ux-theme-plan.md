# ZoneFlow Overlay UX + Theme System — Implementation-Ready Plan

## Status

- **Plan type**: documentation/specification only. No application feature implementation in this task.
- **Repository**: `Rajendertyagi/ZoneFlow`
- **Baseline commit verified**: `7e55c39` (`fix: correct autostart README and delayed-window test timing`, `origin/main` HEAD at audit time)
- **Plan file**: `docs/plans/zoneflow-overlay-ux-theme-plan.md`
- **Supersedes**: prior overlay/theme plan content; this revision locks the decisions and scope below.

## WP completion status (WP6 QA record)

Factual implementation/check record for this plan (no manual verification claimed):

| WP | Status | Commit |
|---|---|---|
| WP0 — audit / spec lock | Implemented (plan committed) | `774465f`, `df7f289`, `6b28435`, `33e195e` |
| WP1 — theme token foundation | Implemented | `65cb6b3` |
| WP2 — preferences / JSON presets | Implemented | `f551425` |
| WP3 — selective primitive a11y | Implemented | `f0c07af` |
| WP4 — theme-aware canvas | Implemented | `c49b42b` |
| WP5 — native overlay UX | Implemented | `3c12e71` |
| WP6 — QA / accessibility / docs | Automated checks run (all passed); docs updated | working tree (uncommitted) |

WP6 automated checks run and passing: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (257 unit + 3 integration), `bun run check`, `bun test` (223 pass), `bun run build`, `bun run themes:check`. Exact commands, exit codes and output were captured in the WP6 review record / PR description (not a repo file); the repo-verifiable evidence artifacts are this status section, [`docs/theme-and-appearance.md`](../theme-and-appearance.md) and [`docs/manual-verification-checklist.md`](../manual-verification-checklist.md).

**Manual Windows/browser verification: NONE PERFORMED.** AC1–AC4, AC7–AC9 and AC10–AC12 remain open and are tracked in [`docs/manual-verification-checklist.md`](../manual-verification-checklist.md). No contrast-ratio checker was run, so AC7 is not established by automated evidence. See [`docs/theme-and-appearance.md`](../theme-and-appearance.md) for the accessibility evidence summary and deferred limitations.

## Baseline Audit (verified against current code)

| Area | Verified current state | Evidence |
|---|---|---|
| App shape | Headless Rust + Axum + embedded Svelte/Vite SPA; browser-on-demand editor; native tray/hotkeys/overlay; embedded assets served by Rust. | `README.md`, `src/web/assets.rs`, `src/api/routes.rs`, `src/native_loop.rs`, `frontend/` |
| Frontend stack | Svelte 5.59.1, Vite 8.3.8, Tailwind 4.3.5, `bits-ui` 2.22.0, `shadcn-svelte` 1.8.1, `paneforge` 1.0.2, `@lucide/svelte`, `tailwind-merge`. No `mode-watcher`. | `frontend/package.json` |
| Theme/config | Single hardcoded dark palette in `frontend/src/app.css` via `@theme` `--color-zf-*` tokens. No theme mode/preset in Rust config, DTOs, API, or frontend settings flow. | `frontend/src/app.css`, `src/config.rs::Settings`, `src/api/dto.rs::SettingsDto/AppOptionsDto/StateSnapshot`, `frontend/src/lib/api/client.ts`, `frontend/src/lib/views/SettingsView.svelte` |
| Settings persistence/dirty | Settings view saves through `PUT /api/settings` + `PUT /api/app-options` (or combined apply endpoint); worker mutates draft, pushes history, marks dirty by draft JSON baseline. | `frontend/src/lib/views/SettingsView.svelte`, `src/api/routes.rs`, `src/api/commands.rs`, `src/api/state.rs::is_dirty/push_history` |
| UI primitives | 14 handrolled files in `frontend/src/lib/components/ui/`: `badge`, `button`, `context-menu`, `dialog`, `input`, `label`, `resizable`, `scroll-area`, `select`, `separator`, `slider`, `switch`, `tabs`, `tooltip`. No Bits UI imports found in `frontend/src`. | `frontend/src/lib/components/ui/*`, grep |
| Canvas/SVG colors | Hardcoded hex colors present in canvas/SVG and app CSS; `zf-*` tokens widely used across frontend. | `frontend/src/app.css`, `frontend/src/lib/components/Canvas/*`, grep |
| Native overlay | Win32 layered overlay painted via `UpdateLayeredWindow`; click-through/non-activating styles (`WS_EX_LAYERED|WS_EX_TRANSPARENT|WS_EX_TOPMOST|WS_EX_TOOLWINDOW|WS_EX_NOACTIVATE`); uses `zone_tints`, `overlay_inactive_alpha`, `overlay_active_alpha`; hover highlight and border emphasis; no labels, no Esc cancel, no snap-success feedback. Gesture is Ctrl+LButton global state; release outside zone returns without sending move command; disabled layout suppresses/cancels overlay. | `src/native_loop.rs` |
| Frozen behavior | D1-D6 command/state semantics, dirty guard, undo/redo snapshot model, profile dimensions, and existing API contracts are in place and must be preserved. | `src/api/commands.rs`, `src/api/state.rs`, tests |

## Locked Decisions

These decisions are final for this plan and must not be reopened during implementation without a new approved plan revision.

1. **Appearance preferences storage**: use ModeWatcher persistent preferences/local browser storage for theme mode and preset. **Do not** add theme mode/preset to workspace `Settings`/`SettingsDto`, `AppOptionsDto`, `AppConfig`, or any Rust workspace configuration field.
2. **No dirty/undo impact**: appearance preference changes must not create draft dirty state, must not affect Save/Discard, and must not create undo/redo history.
3. **No new Rust theme-settings API**: first implementation adds no new Rust endpoints/commands/DTO fields for theme preferences.
4. **Local scope**: localStorage preferences are per browser profile/device. The plan must state this explicitly in UI/docs.
5. **ModeWatcher initialization (locked)**: use documented ModeWatcher storage keys `mode-watcher-mode` and `mode-watcher-theme`. The custom early bootstrap in `frontend/index.html` is the **single pre-paint initialization owner**. It reads and validates both stored values; resolves `system` using the OS color preference (`prefers-color-scheme`); sets the `.dark` class, `color-scheme`, and `data-theme` on `<html>` before the app paints. `ModeWatcher` in `App.svelte` uses `disableHeadScriptInjection={true}` so it cannot introduce a competing initialization script. Subsequent user changes use ModeWatcher functions and remain synchronized with the same keys.
6. **Theme JSON / generated CSS (locked)**: built-in JSON preset files are the source of truth. Complete static CSS for every preset and mode is generated during the frontend build; no second manually duplicated copy of preset token values in the bootstrap script. Exact typed schema and allowed token keys; built-in presets only in first release; no arbitrary user CSS and no custom theme import. Validate actual preset fixtures. Invalid preset IDs fall back to `zoneflow-amber`. Bun is a build tool only, not a production runtime.
7. **Design tokens**: inventory every existing `zf-*` token and hardcoded color; define one exact semantic mapping for each existing token; no ambiguous mappings; define foreground/background pairs, focus, border, destructive, success, warning, and canvas-specific tokens; keep existing aliases until migration is verified complete.
8. **UI primitives**: do not replace every custom component. Provide component-by-component inventory with current behavior, accessibility gaps, recommended action, and acceptance criteria. Preserve native HTML controls/simple wrappers where appropriate. Evaluate dialog, context menu, tabs, tooltip, resizable, switch individually. Do not blindly overwrite current `.svelte` components with generated shadcn-svelte files. Preserve currently installed compatible dependency versions unless a specific upgrade is justified and documented.
9. **Native overlay**: document existing non-activating, click-through behavior; Escape polling during active gesture uses a `cancelled_until_release` latch; Escape must cancel without sending window-move command; releasing outside zones must not send move command; preserve disabled-layout suppression; define exactly how active target emphasis and native zone labels work through an explicit internal native-only payload extension; do not promise snap-success feedback unless confirmed worker results can reach native UI; defer keyboard zone selection until separately specified; keep overlay color settings independent of editor theme preferences in first implementation.
10. **Scope control**: focus on theme system/appearance preferences, semantic token migration, selective UI accessibility modernization, canvas visual consistency, native overlay UX improvements, testing/docs. Move profile import/export, profile reordering, app-picker enhancements, hotkey recording, and unrelated workflow features to separate future backlog.
11. **Architecture preserved**: Rust + Axum + embedded Svelte/Vite frontend, authoritative worker state model, current Windows native message loop, existing embedded asset build pipeline. No Electron/Tauri/another frontend server/production Node-Bun runtime/auth/Host-Origin validation/CSRF/CORS allowlists. Preserve all frozen D1-D6 behavior. Do not modify unrelated source files.

## Official References

- shadcn-svelte Theming: https://shadcn-svelte.com/docs/theming
- shadcn-svelte Dark Mode (Svelte): https://shadcn-svelte.com/docs/dark-mode/svelte
- mode-watcher docs: https://mode-watcher.svecosystem.com/docs
- mode-watcher ModeWatcher: https://mode-watcher.svecosystem.com/docs/components/mode-watcher
- mode-watcher theme state: https://mode-watcher.svecosystem.com/docs/states/theme
- tweakcn/shadcn theme JSON example: https://tweakcn.com/r/themes/caffeine.json

## Scope

### In scope

- Theme mode (`system | light | dark`) and built-in preset selection via ModeWatcher persistent preferences.
- Semantic token foundation, `zf-*` compatibility aliases, and tokenized canvas/SVG styling.
- Selective UI primitive modernization for accessibility and consistency, without wholesale replacement.
- Native overlay UX improvements: Escape cancel, active-target emphasis, zone labels, and preserved existing safety behavior.
- Tests, manual acceptance criteria, accessibility checks, and documentation.

### Out of scope / future backlog

- Profile import/export, profile reordering, quick-switch redesign beyond existing behavior.
- App picker metadata/recent/missing badges enhancements.
- Hotkey recording/conflict UX redesign.
- Custom theme import, arbitrary user CSS, remote theme loading, theme marketplace.
- Keyboard zone selection during drag (deferred until separately specified).
- Snap-success feedback that depends on worker-to-native UI confirmation path not currently present.
- Any unrelated workflow feature or refactor.

## Current-State Inventory

### `zf-*` token inventory (current `frontend/src/app.css`)

| Token | Current value | Exact semantic mapping | Notes |
|---|---:|---|---|
| `--color-zf-bg` | `#070b14` | `--background` | App canvas/background. |
| `--color-zf-surface` | `#0d1526` | `--card` | Default panel/card surface. |
| `--color-zf-panel` | `#111c31` | `--popover` | Elevated panel/dialog/popover surface. |
| `--color-zf-raised` | `#17243d` | `--secondary` | Raised/hover/selected neutral surface; not muted/accent ambiguous. |
| `--color-zf-border` | `#22304a` | `--border` | Subtle border/hairline. |
| `--color-zf-border-strong` | `#33445f` | `--input` | Strong affordance border/input border. |
| `--color-zf-text` | `#e6edf7` | `--foreground` | Primary text. |
| `--color-zf-muted` | `#93a4bd` | `--muted-foreground` | Secondary text. |
| `--color-zf-faint` | `#64748b` | `--muted-foreground-faint` | Exact custom semantic token; defined in every preset/mode fixture and generated CSS; `--color-zf-faint` remains a compatibility alias until migration complete. |
| `--color-zf-accent` | `#f59e0b` | `--primary` | Primary action/accent. |
| `--color-zf-accent-hover` | `#fbbf24` | `--primary-hover` (custom semantic token) | Explicit token; no ad-hoc hover color. |
| `--color-zf-accent-ink` | `#1a1204` | `--primary-foreground` | Text/icon on primary. |
| `--color-zf-ok` | `#34d399` | `--success` | Success/status token. |
| `--color-zf-warn` | `#fbbf24` | `--warning` | Warning/status token. |
| `--color-zf-error` | `#f87171` | `--destructive` | Destructive/error token. |

### Required semantic token set (minimum)

Define in generated preset CSS and register through Tailwind v4 `@theme inline`, and consume via semantic utilities/classes:

- Surfaces/text: `background`, `foreground`, `card`, `card-foreground`, `popover`, `popover-foreground`, `secondary`, `secondary-foreground`, `muted`, `muted-foreground`, `muted-foreground-faint`.
- Actions/focus: `primary`, `primary-hover`, `primary-foreground`, `accent`, `accent-foreground`, `ring`.
- Borders/inputs: `border`, `input`.
- Status: `destructive`, `destructive-foreground`, `success`, `success-foreground`, `warning`, `warning-foreground`.
- Canvas-specific: `canvas-bg`, `canvas-grid`, `canvas-zone-stroke`, `canvas-zone-selected`, `canvas-zone-drop`, `canvas-zone-merge`, `canvas-handle-shared`, `canvas-handle-outer`, `canvas-preview`, `canvas-label-bg`, `canvas-label-fg`.
- Optional chart tokens: `chart-1`..`chart-5`.

No component may map a `zf-*` token to multiple semantic targets. Compatibility aliases may point old `zf-*` names to the new semantic tokens until removal. No overlay-specific preset tokens are included; native overlay colors remain independent workspace settings.

### Hardcoded color inventory (must be tokenized or explicitly justified)

| Location | Current hardcoded colors | Action |
|---|---|---|
| `frontend/src/app.css` | `#070b14`, `#0d1526`, `#111c31`, `#17243d`, `#22304a`, `#33445f`, `#e6edf7`, `#93a4bd`, `#64748b`, `#f59e0b`, `#fbbf24`, `#1a1204`, `#34d399`, `#fbbf24`, `#f87171` | Replace with semantic vars + preset values; keep aliases during migration. |
| `frontend/src/lib/views/SettingsView.svelte` | default `zone_tints` hex array | Keep as overlay settings defaults; independent from editor theme preset. |
| `frontend/src/lib/components/Canvas/ZoneCanvas.svelte` | grid stroke `#22304a`, empty-state text `#93a4bd/#6b7c96`, fallback tint `#f59e0b` | Replace with canvas semantic tokens/CSS vars. |
| `frontend/src/lib/components/Canvas/ZoneRect.svelte` | strokes/fills `#f59e0b`, `#c792ea`, `#33445f`, badge bg `#0b1322`, text `#e6edf7/#93a4bd/#1a1204`, handles `#6EA8FE/#f59e0b` | Replace with canvas semantic tokens; preserve meaning of selected/drop/merge/shared/outer handles. |
| `frontend/src/lib/components/Canvas/DragPreview.svelte` | preview fill/stroke/text `#f59e0b/#fbbf24` | Replace with `canvas-preview` tokens. |
| `frontend/src/lib/components/Canvas/SnapGuides.svelte` | guide/hud `#f59e0b/#0b1322/#fbbf24` | Replace with exact mappings: guide strokes -> `canvas-preview`; HUD background -> `canvas-label-bg`; HUD border -> `canvas-preview`; HUD text -> `canvas-label-fg`. |
| `frontend/src/lib/components/ui/*` | `bg-white`, `bg-black/70` in dialog/switch as applicable | Evaluate: keep only if mapped to semantic overlay/switch-thumb tokens or documented exception. |

## Theme System Design

### Mode and preset ownership

- **Mode**: `system | light | dark`, managed by `mode-watcher`.
- **Preset**: built-in preset ID string, managed by ModeWatcher theme state/localStorage.
- **Storage keys (locked)**: `mode-watcher-mode` and `mode-watcher-theme`.
- **Authoritative source for appearance**: browser-local ModeWatcher/preferences state, not workspace config.
- **Workspace config**: unchanged for theme mode/preset.

### File layout

```text
frontend/src/themes/
  schema.ts                 # literal key types, runtime allowlists, normalization, validation
  presets/
    zoneflow-amber.json     # built-in preset fixture (source of truth)
    caffeine.json           # built-in preset fixture (source of truth)
  manifest.ts               # built-in manifest metadata (id, label, swatches), default/fallback
  generate-css.ts           # build script: JSON fixtures -> generated static CSS
  generated/
    presets.css             # generated output; committed or reproducibly generated and checked
  schema.test.ts            # fixture validation/normalization/fallback tests
  generated-css.test.ts     # verifies generated CSS is current and valid
frontend/src/lib/theme/
  preference.ts             # ModeWatcher key constants/helpers, fallback validation
  preference.test.ts
frontend/index.html         # WP2: single pre-paint bootstrap owner (no preset token duplication)
frontend/src/main.ts        # imports app.css; no competing theme init
frontend/src/App.svelte     # mounts ModeWatcher once with disableHeadScriptInjection={true}
frontend/src/app.css        # imports generated/presets.css and defines semantic/alias mappings
```

Built-in preset JSON files are the source of truth. Complete static CSS for every preset and mode is generated during the frontend build. Bun is a build tool only, not a production runtime.

### Exact theme JSON schema (built-in presets)

```ts
export const SHARED_THEME_KEYS = [
  'radius',
  'font-sans',
  'font-mono',
] as const;

export const REQUIRED_LIGHT_DARK_KEYS = [
  'background', 'foreground',
  'card', 'card-foreground',
  'popover', 'popover-foreground',
  'primary', 'primary-hover', 'primary-foreground',
  'secondary', 'secondary-foreground',
  'muted', 'muted-foreground', 'muted-foreground-faint',
  'accent', 'accent-foreground',
  'destructive', 'destructive-foreground',
  'success', 'success-foreground',
  'warning', 'warning-foreground',
  'border', 'input', 'ring',
  'canvas-bg', 'canvas-grid', 'canvas-zone-stroke', 'canvas-zone-selected',
  'canvas-zone-drop', 'canvas-zone-merge', 'canvas-handle-shared',
  'canvas-handle-outer', 'canvas-preview', 'canvas-label-bg', 'canvas-label-fg',
] as const;

export const OPTIONAL_LIGHT_DARK_KEYS = [
  'chart-1', 'chart-2', 'chart-3', 'chart-4', 'chart-5',
] as const;

export const LIGHT_DARK_KEYS = [
  ...REQUIRED_LIGHT_DARK_KEYS,
  ...OPTIONAL_LIGHT_DARK_KEYS,
] as const;

export type SharedThemeKey = typeof SHARED_THEME_KEYS[number];
export type RequiredLightDarkKey = typeof REQUIRED_LIGHT_DARK_KEYS[number];
export type OptionalLightDarkKey = typeof OPTIONAL_LIGHT_DARK_KEYS[number];
export type LightDarkKey = typeof LIGHT_DARK_KEYS[number];

export type ThemePresetFile = {
  $schema?: string; // optional metadata, ignored by runtime/generator after validation
  name: string;     // stable preset id, kebab-case; used as data-theme value
  type: 'registry:style';
  cssVars: {
    theme: Record<SharedThemeKey, string>;
    light: Record<RequiredLightDarkKey, string> & Partial<Record<OptionalLightDarkKey, string>>;
    dark: Record<RequiredLightDarkKey, string> & Partial<Record<OptionalLightDarkKey, string>>;
  };
};
```

**Manifest metadata clarification**: `label` (and optional swatches/description) is ZoneFlow manifest metadata in `manifest.ts`, not part of the normalized registry-style preset JSON file. Preset JSON files remain tweakcn-compatible in shape (`name`, `type`, `cssVars`).

### Required vs optional light/dark tokens

**Required in both `light` and `dark`**:
- `background`, `foreground`, `card`, `card-foreground`, `popover`, `popover-foreground`, `primary`, `primary-hover`, `primary-foreground`, `secondary`, `secondary-foreground`, `muted`, `muted-foreground`, `accent`, `accent-foreground`, `destructive`, `destructive-foreground`, `success`, `success-foreground`, `warning`, `warning-foreground`, `border`, `input`, `ring`, `canvas-bg`, `canvas-grid`, `canvas-zone-stroke`, `canvas-zone-selected`, `canvas-zone-drop`, `canvas-zone-merge`, `canvas-handle-shared`, `canvas-handle-outer`, `canvas-preview`, `canvas-label-bg`, `canvas-label-fg`.

**Optional in both `light` and `dark`** (must still be in allowlist if present):
- `chart-1`, `chart-2`, `chart-3`, `chart-4`, `chart-5`.

**Required shared theme keys**: `radius`, `font-sans`, `font-mono`.

No wildcard key declarations (`Record<string, string>`) are allowed in the schema/types or runtime allowlists.

### Custom ZoneFlow-only tokens (explicit)

These are ZoneFlow extensions to the shadcn/tweakcn-compatible base and must be explicitly included in schema, fixtures, generated CSS, and tests:
- `primary-hover`
- `success`, `success-foreground`
- `warning`, `warning-foreground`
- `canvas-bg`, `canvas-grid`, `canvas-zone-stroke`, `canvas-zone-selected`, `canvas-zone-drop`, `canvas-zone-merge`, `canvas-handle-shared`, `canvas-handle-outer`, `canvas-preview`, `canvas-label-bg`, `canvas-label-fg`

### `zf-faint` semantic destination (locked)

`--color-zf-faint` maps exactly to `--muted-foreground-faint` (custom semantic alias token). `--muted-foreground-faint` is defined in generated preset CSS for every preset/mode and registered in `@theme inline` as `--color-muted-foreground-faint` / compatibility alias as needed. It is used only for tertiary/de-emphasized text and must pass the contrast rule for its intended use (see Accessibility). It is included in fixture schema/tests as a required light/dark token.

Native overlay colors remain independent and are not represented in preset JSON.

### Overlay independence

No overlay-specific preset tokens are included in theme JSON. Native overlay continues to use existing workspace settings (`zone_tints`, `overlay_inactive_alpha`, `overlay_active_alpha`) and native style messages.

### Normalization and validation

- Allow only keys in `SHARED_THEME_KEYS` and `LIGHT_DARK_KEYS`; unknown keys are ignored with dev warning.
- Missing required key => preset fixture invalid (build/test failure); runtime loader falls back to `zoneflow-amber`.
- Invalid value (empty, non-string, or obviously malformed for the token type) => preset fixture invalid.
- Deduplicate across `theme/light/dark`: shared tokens in `theme`; mode tokens in `light/dark`. If a key appears in both, mode-specific value wins and normalization records a dev warning.
- Trim strings; preserve authored color format.
- `css` blocks are not applied. Built-in fixtures must not include `css`; generator/validator rejects them.

### Generated static CSS (no runtime duplication)

- **Generation script**: `frontend/src/themes/generate-css.ts` (run via Bun during frontend build).
- **Output**: `frontend/src/themes/generated/presets.css`.
- **Source of truth**: only `frontend/src/themes/presets/*.json` + schema/manifest.
- **`app.css` import**: `frontend/src/app.css` imports generated CSS (e.g. `@import './themes/generated/presets.css';`) before/alongside semantic alias mappings as appropriate.
- **Selectors**: generated CSS emits complete token blocks per preset and mode using `data-theme` and `.dark`:
  - `:root[data-theme="zoneflow-amber"] { /* light tokens */ }`
  - `:root[data-theme="zoneflow-amber"].dark { /* dark tokens */ }`
  - equivalent blocks for every built-in preset.
- **Completeness**: generated CSS contains every required token for every preset/mode; no missing-key fallback to another preset inside generated output except explicit default selection.
- **Build/test verification**:
  - `bun run build` (or dedicated script) runs generation before Vite build.
  - Test verifies `generated/presets.css` is up-to-date with fixtures (regenerate and compare, or hash/content check).
  - Test validates generated CSS contains required selectors/tokens for every built-in preset and both modes.
  - CI/local check fails if generated output is stale or invalid.
- **Invalid preset ID fallback**: runtime/bootstrap/manifest lookup falls back to `zoneflow-amber`; `data-theme` is set to `zoneflow-amber` when stored value is invalid.

### Pre-paint initialization (single owner, WP2)

- `frontend/index.html` contains one small inline script before module script. It:
  - reads `mode-watcher-mode` and `mode-watcher-theme` from localStorage;
  - validates mode against `light | dark | system`; validates preset ID against built-in allowlist;
  - resolves `system` using `window.matchMedia('(prefers-color-scheme: dark)')`;
  - sets `<html>.dark` class when resolved mode is dark;
  - sets `<html>.style.colorScheme` to `light` or `dark`;
  - sets `<html>.dataset.theme` to validated preset ID (or `zoneflow-amber` fallback);
  - does **not** duplicate preset token values; generated CSS provides them.
- `ModeWatcher` in `App.svelte` uses `disableHeadScriptInjection={true}` and is configured with the same storage keys so it cannot introduce a competing initialization script.
- Subsequent user changes use ModeWatcher functions (`setMode`/`toggleMode`/theme setter as applicable) and remain synchronized with the same keys.
- `main.ts` does not initialize theme. No other script mutates theme before paint.

## UI Primitive Inventory (selective modernization)

| Component | Current behavior | Accessibility/UX gaps | Recommended action | Acceptance criteria |
|---|---|---|---|---|
| `button.svelte` | Variants/sizes, focus-visible ring, disabled styling. | No loading state; relies on custom classes. | Keep wrapper; restyle to semantic tokens; optionally align API with shadcn pattern without breaking call sites. | All existing variants/sizes work; focus-visible visible in light/dark; contrast AA. |
| `input.svelte` | Text input with bindable value, ariaLabel. | No error/describedby pattern; native styling only. | Keep native input wrapper; tokenize; add optional `aria-invalid`/describedby support if needed. | Keyboard/focus/disabled behavior unchanged; labels/readability AA. |
| `select.svelte` | Native `<select>`. | Limited styling; acceptable semantics. | Keep native select for first implementation. | Preserves keyboard/native behavior; tokenized; no regression. |
| `switch.svelte` | Button role=switch, aria-checked, keyboard click. | No label association pattern beyond aria-label; thumb is white. | Keep custom or replace with Bits UI Switch only if API migration is low-risk; evaluate individually. | Space/Enter/click toggles; aria-checked correct; focus-visible; contrast AA. |
| `dialog.svelte` | Modal overlay, Escape close, backdrop click, aria-modal. | No focus trap, no focus restore, no aria-describedby, no scroll lock management beyond overlay. | Replace/adapt using Bits UI Dialog primitives or shadcn-svelte dialog pattern; preserve simple API used by call sites. | Focus trapped, Escape closes, focus restored, backdrop click behavior defined, screen reader name/description correct. |
| `context-menu.svelte` | Right-click custom menu, click-outside/Escape close. | No keyboard navigation, no roving focus, no typeahead, no submenu semantics, menu positioned by viewport click only. | Replace with Bits UI/shadcn-svelte Context Menu; preserve `items` API via adapter if feasible. | Keyboard open/navigate/activate/close works; focus managed; ARIA menu roles correct. |
| `tabs.svelte` | Clickable tab buttons with aria-selected/tablist. | No arrow key navigation, no tabpanel id/aria-controls linkage, no Home/End. | Replace with Bits UI Tabs or add full keyboard semantics. | Arrow/Home/End navigation, aria-controls/tabpanel linkage, focus-visible. |
| `tooltip.svelte` | Hover-only tooltip. | No focus trigger, no aria-describedby, no touch/escape semantics, hover-only. | Replace with Bits UI Tooltip or add focus/ARIA support. | Visible on focus and hover; dismissible; aria-describedby wired; not hover-only. |
| `resizable.svelte` | Custom pointer split pane. | Keyboard resize not present; semantics limited. | Keep custom for first implementation; document limitation; evaluate Bits/Paneforge only if needed. | Mouse/touch pointer resize unchanged; focusable handle if added later; no regression. |
| `scroll-area.svelte` | Simple overflow container with scrollbar styling. | No semantic issue; not full shadcn scroll area. | Keep simple wrapper; tokenize scrollbar/colors. | Scrolling unchanged; contrast/readability OK. |
| `separator.svelte` | Decorative separator with role. | Minor: ensure orientation semantics used correctly. | Keep. | Correct role/orientation; tokenized. |
| `slider.svelte` | Native range input. | Native semantics good; styling limited. | Keep native range for first implementation. | Keyboard/native behavior unchanged; tokenized. |
| `badge.svelte` | Status variants. | Color-only status risk in some contexts. | Keep; tokenize status colors and ensure text/contrast. | Status readable in all presets/modes; AA contrast. |
| `label.svelte` | Simple label wrapper. | None major. | Keep. | Tokenized; association unchanged. |

## Native Overlay UX Design

### Existing behavior to preserve

- Overlay window is non-activating and click-through: `WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE` (`src/native_loop.rs`).
- Gesture detection uses global Ctrl+LButton state; disabled layout suppresses/cancels overlay (`gesture_step`, `enabled`).
- Drag release outside any zone returns without sending `MoveWindow` (`on_drag_release` checks `zone_at` and returns).
- Overlay style is driven by settings (`zone_tints`, `overlay_inactive_alpha`, `overlay_active_alpha`) via `SetOverlayStyle`/`sync_overlay_style`.

### Escape cancellation (locked)

The native loop polls Ctrl+LeftButton. Simply clearing the active flag on Escape is insufficient because the next iteration could start the same gesture again while Ctrl+LButton remain held.

**Required latch**: `cancelled_until_release: bool` (native main-thread state).

**Behavior**:
- **Normal gesture start**: when enabled, not active, not cancelled, and Ctrl+LButton are down, begin gesture and show overlay.
- **Escape during active gesture**: if Escape is detected while active, set `cancelled_until_release = true`, hide overlay, clear active/hover state, and do **not** send `MoveWindow`. Escape takes priority over normal gesture-end handling when cancellation is detected.
- **Continued Ctrl+LeftButton after Escape**: while `cancelled_until_release` is true, gesture remains suppressed; no overlay show/update and no `MoveWindow`.
- **Release of Ctrl or LeftButton**: when either Ctrl or LeftButton is released, clear `cancelled_until_release` (and ensure no drop occurs for the cancelled gesture).
- **Subsequent fresh gesture**: next gesture starts only after release and a new Ctrl+LButton down edge while enabled and not cancelled.
- **No `MoveWindow` command during cancelled gesture**: guaranteed by state machine; cancel path never calls drop/send.

**Required tests** (pure/state-machine where possible, plus manual Windows verification):
- Normal gesture start.
- Escape during an active gesture.
- Continued Ctrl+LeftButton after Escape.
- Release of Ctrl or LeftButton.
- A subsequent fresh gesture.
- No `MoveWindow` command during the cancelled gesture.

### Native zone-label data path (locked)

Current `SetZones` payload and `Inner::drag_zones()` contain geometry and IDs, not labels. Named zone labels require an explicit internal payload extension.

**Required internal structure** (native-only, not HTTP/DTO/workspace config):
```rust
struct OverlayZone {
    id: i32,
    rect: Rect,
    label: String, // short display label; may be empty
}
```

**Data flow**:
- Worker constructs `Vec<OverlayZone>` from the currently selected profile's zones using the same geometry calculation as `Inner::drag_zones()` (same selected profile, same work area/gap/zone geometry source).
- Startup sends initial overlay zones including labels.
- Subsequent `SetZones` refreshes include labels whenever zone/profile/selection state changes that affect overlay zones.
- Native renderer draws a 1-based index badge and a suitably short zone name (truncate/ellipsize as needed for readability; never obscure hit target).
- **Authority boundary (locked)**: native cached overlay zones are used for visualization and hit-testing only. On drag release, native sends the selected `zone_id` to the worker; the worker independently resolves that ID against `Inner::drag_zones()`/authoritative worker state and performs the final `MoveWindow` decision. Both native cache and worker resolution must derive from the same selected profile and geometry calculation. The native cached rectangle is never authoritative for the final move.

**Constraints**:
- No HTTP endpoint, workspace DTO, theme API, or persisted field for this.
- No theme-preset coupling; label colors use existing overlay/tint/contrast-safe rendering rules.

### Planned behavior

| Behavior | Specification | Verification |
|---|---|---|
| Escape cancel | `cancelled_until_release` latch; Escape takes priority; no `MoveWindow`; no restart until release and fresh gesture. | Pure state tests + manual Windows test. |
| Release outside zone | Keep current behavior: no `MoveWindow` command sent. | Code inspection + manual Windows test. |
| Disabled layout suppression | Keep current `gesture_step` behavior; no overlay shown while disabled; in-flight drag cancels without drop. | Existing tests + manual Windows test. |
| Active target emphasis | Hovered zone gets stronger border/alpha/label emphasis; inactive zones dimmed. Use existing alpha settings and tint palette; no theme-preset dependency. | Manual Windows visual test. |
| Zone labels | Draw 1-based index and short zone name from native-only overlay-zone payload; GDI text/simple lightweight rendering. | Manual Windows visual test; code inspection. |
| Snap-success feedback | **Not promised** in first implementation unless worker result can be confirmed to reach native UI. No fake success indicator. | Code review; omit from acceptance unless implemented with evidence. |
| Keyboard zone selection | Deferred until separately specified. | N/A |

### Overlay/theme independence

- Editor theme mode/preset changes must not alter native overlay `zone_tints`, `overlay_inactive_alpha`, or `overlay_active_alpha`.
- Native overlay remains controlled by existing workspace settings and native style messages.

## Work Packages

### WP0 — Audit and final spec lock

- **Scope**: Verify baseline, inventory tokens/colors/components, confirm official docs, finalize this plan.
- **Non-goals**: No feature code.
- **Dependencies**: None.
- **Exact files/areas**: `docs/plans/zoneflow-overlay-ux-theme-plan.md`; read-only audit of files listed in Baseline Audit.
- **Data flow/state ownership**: Documentation only.
- **Behavior**: Plan contains no unresolved architecture decisions; all claims grounded in code/docs.
- **Failure/fallback**: If a claim cannot be verified, mark as assumption/risk; do not present as fact.
- **Automated tests**: N/A (doc task); verify commands listed in Verification Plan when implementation begins.
- **Manual acceptance**: Reviewer can trace every major claim to a file path or official doc.
- **Completion criteria**: Plan committed and pushed; adversarial review completed; valid findings fixed.
- **Regression risks**: Stale assumptions causing rework; mitigated by code-grounded audit.

### WP1 — Theme token foundation

- **Scope**: Introduce semantic CSS variables, Tailwind v4 `@theme inline` mappings, `zf-*` compatibility aliases, canvas semantic tokens, preset JSON schema/fixtures, and complete generated static CSS for every preset/mode.
- **Non-goals**: No ModeWatcher UI, no `index.html` bootstrap changes (WP2 only), no Rust config/DTO changes, no user import/CSS.
- **Dependencies**: WP0.
- **Exact files/areas**: `frontend/src/app.css`, `frontend/src/themes/schema.ts`, `frontend/src/themes/presets/*.json`, `frontend/src/themes/manifest.ts`, `frontend/src/themes/generate-css.ts`, `frontend/src/themes/generated/presets.css`, `frontend/src/themes/schema.test.ts`, `frontend/src/themes/generated-css.test.ts`, `frontend/package.json` scripts as needed for generation/check, `frontend/src/main.ts` (no competing init).
- **Data flow/state ownership**: Built-in preset JSON fixtures -> validate/normalize -> generate complete static CSS. No runtime duplication of preset token values. No workspace draft/state.
- **Behavior**: Default preset is `zoneflow-amber`; invalid preset ID falls back to `zoneflow-amber`; generated CSS contains complete required token blocks for every preset and both modes; selectors use `:root[data-theme="..."]` and `:root[data-theme="..."].dark`.
- **Failure/fallback**: Invalid token/key/value => fixture/test failure; unknown keys ignored with dev warning; invalid preset ID at runtime falls back to default.
- **Automated tests**: Fixture validation tests for every built-in preset; normalization/dedupe/fallback unit tests; generated CSS current/valid tests (stale check + required selector/token presence).
- **Manual acceptance**: App renders with default theme; no missing token regressions in light/dark baseline.
- **Completion criteria**: All `zf-*` tokens mapped exactly once; aliases present; preset fixtures pass; generated CSS is current and valid; no `index.html` changes in WP1.
- **Regression risks**: Visual regressions from token remap; mitigate by alias layer and visual QA.

### WP2 — Theme preferences and JSON presets

- **Scope**: Add ModeWatcher, appearance controls, local preference persistence, pre-paint bootstrap, preset switching.
- **Non-goals**: No workspace Settings/DTO/API changes; no dirty/undo/history; no custom import; no manual duplication of preset token values in bootstrap.
- **Dependencies**: WP1.
- **Exact files/areas**: `frontend/package.json` (add `mode-watcher`, pinned compatible version; no unrelated dependency upgrades), `frontend/src/App.svelte`, `frontend/index.html`, `frontend/src/main.ts`, `frontend/src/lib/theme/preference.ts`, `frontend/src/lib/views/SettingsView.svelte` or dedicated appearance component, `frontend/src/lib/components/*` as needed for appearance UI.
- **Data flow/state ownership**: ModeWatcher/localStorage owns mode/preset using `mode-watcher-mode` and `mode-watcher-theme`. UI reads/writes appearance prefs only; does not call settings save endpoints for appearance.
- **Behavior**: `index.html` bootstrap is the single pre-paint owner: reads/validates both keys, resolves `system` via `prefers-color-scheme`, sets `.dark`, `color-scheme`, and `data-theme` before paint. `ModeWatcher` in `App.svelte` uses `disableHeadScriptInjection={true}` and same keys; subsequent changes use ModeWatcher functions and stay synchronized. Mode/preset apply instantly; survive reload/restart; system mode follows OS; no FOUC; per-browser-profile scope stated in UI. If appearance controls live in `SettingsView`, they must be isolated from workspace settings draft/save flow and must not trigger `setSettings`/`setAppOptions`/`save`/undo history.
- **Failure/fallback**: localStorage unavailable/corrupt => defaults; invalid stored values => `system`/`zoneflow-amber`.
- **Automated tests**: preference parse/fallback tests; component/state tests for appearance controls where feasible.
- **Manual acceptance**: Hard refresh, first launch, system mode change, preset switch, restart persistence; workspace dirty/save/undo unaffected.
- **Completion criteria**: All WP2 acceptance criteria pass; no Rust API/config changes; no competing initialization script.
- **Regression risks**: FOUC or ModeWatcher/bootstrap conflict; mitigate with single init strategy and manual checks.

### WP3 — Selective UI primitive modernization

- **Scope**: Accessibility and token modernization for selected primitives; replace only where justified.
- **Non-goals**: No wholesale generated overwrite; no unrelated visual redesign.
- **Dependencies**: WP1.
- **Exact files/areas**: `frontend/src/lib/components/ui/*` (selected), call sites in views/components/dialogs, `frontend/components.json` (only if shadcn-svelte alignment requires it), `frontend/src/lib/utils.ts` if needed, tests.
- **Data flow/state ownership**: Component-local/UI state only; no workspace draft changes.
- **Behavior**: Dialog/context menu/tabs/tooltip/switch meet accessibility acceptance criteria; native select/slider/resizable/scroll-area/separator may remain wrappers if criteria met.
- **Failure/fallback**: If replacement risks regression, keep wrapper and add missing semantics.
- **Automated tests**: Component/unit tests for keyboard/focus behavior where feasible; existing frontend tests pass.
- **Manual acceptance**: Keyboard-only navigation, visible focus, Escape/Tab/arrow behavior as applicable, screen-reader labels/roles for modernized primitives; reduced-motion check for any motion added/changed.
- **Completion criteria**: Inventory table actions completed or explicitly deferred with justification; no behavior regressions; deferred accessibility limitations documented explicitly.
- **Regression risks**: API mismatch with existing call sites; mitigate by adapter wrappers and incremental migration.

### WP4 — Theme-aware canvas

- **Scope**: Tokenize canvas/SVG colors, improve readability across presets/modes, keep geometry/interaction behavior unchanged.
- **Non-goals**: No canvas feature redesign, no new workflow features.
- **Dependencies**: WP1, WP3 (for shared tokens/primitives where applicable).
- **Exact files/areas**: `frontend/src/lib/components/Canvas/*`, `frontend/src/app.css`, canvas-related view styles.
- **Data flow/state ownership**: Visual-only; zone geometry/state remains authoritative through existing stores/API.
- **Behavior**: Grid, zones, handles, previews, guides, labels, empty states readable in all built-in presets and modes.
- **Failure/fallback**: If a token misses contrast, adjust preset/token values and revalidate.
- **Automated tests**: Existing frontend tests; add snapshot/unit tests only where stable/useful.
- **Manual acceptance**: Canvas readable in light/dark and every preset; selected/drop/merge/handle states distinguishable.
- **Completion criteria**: No hardcoded canvas hex except documented exception; visual QA pass.
- **Regression risks**: Reduced affordance in borderless style; mitigate with contrast/focus/handle checks.

### WP5 — Native overlay UX

- **Scope**: Escape cancel with `cancelled_until_release` latch, active-target emphasis, zone labels via explicit internal native-only payload extension; preserve existing overlay safety behavior.
- **Non-goals**: No keyboard zone selection; no snap-success feedback unless worker confirmation path is implemented; no theme-preset coupling; no HTTP endpoint/workspace DTO/theme API/persisted field for labels.
- **Dependencies**: WP0; independent of WP2-WP4 except shared release QA.
- **Exact files/areas**: `src/native_loop.rs`, possibly `src/api/commands.rs`/`src/api/state.rs`/native message payloads only for internal overlay-zone label payload (not a theme-preference API); tests in `src/native_loop.rs`.
- **Data flow/state ownership**: Native main thread owns overlay/gesture state and uses cached overlay zones for visualization/hit-testing only. Worker remains authoritative for final move command execution: native sends `zone_id`, worker independently resolves it against `Inner::drag_zones()`/authoritative state. Both derive from the same selected profile and geometry calculation; native cached rectangle is never authoritative.
- **Behavior**: Esc sets `cancelled_until_release`, hides overlay, clears active/hover state, and prevents `MoveWindow`; Escape takes priority over normal gesture-end handling. While cancelled, continued Ctrl+LButton does not restart gesture. Release of Ctrl or LeftButton clears latch; next fresh gesture requires new down edge. Release outside zone no move; disabled layout suppresses/cancels; hovered zone clearly emphasized; overlay draws 1-based index and short zone name.
- **Failure/fallback**: If label rendering is unreliable, ship index badge/minimal label with documented limitation; do not block Escape/cancel safety.
- **Automated tests**: Pure/state-machine tests for normal start, Escape during active gesture, continued Ctrl+LButton after Escape, release of Ctrl or LeftButton, subsequent fresh gesture, and no `MoveWindow` during cancelled gesture.
- **Manual acceptance**: Windows drag-snap tests for Esc cancel, outside release, disabled layout, active target emphasis, labels.
- **Completion criteria**: All WP5 acceptance criteria pass; no regression in existing overlay tests.
- **Regression risks**: Input polling interfering with gesture; mitigate by narrow state transitions and tests.

### WP6 — Full QA, accessibility, docs

- **Scope**: End-to-end verification, accessibility checks, docs updates, final review.
- **Non-goals**: No new features.
- **Dependencies**: WP1-WP5.
- **Exact files/areas**: `README.md` or relevant docs, `docs/plans/zoneflow-overlay-ux-theme-plan.md` (if status updates needed), test files, release checklist notes.
- **Data flow/state ownership**: N/A.
- **Behavior**: All acceptance criteria verified with evidence; docs describe theme location/format, per-browser-profile scope, and overlay independence.
- **Failure/fallback**: Any failed criterion blocks completion.
- **Automated tests**: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cd frontend && bun run check && bun test && bun run build`.
- **Manual acceptance**: Windows/browser checklist below.
- **Completion criteria**: All required checks pass or failures are explicitly reported; no unverified manual claims.
- **Regression risks**: Missing platform-specific manual verification; mitigate by explicit reporting.

## Measurable Acceptance Criteria

| ID | Criterion | Verification method |
|---|---|---|
| AC1 | Mode (`light/dark/system`) and preset survive reload/restart in same browser profile using `mode-watcher-mode` and `mode-watcher-theme`. | Manual browser test: set values, reload, restart app/browser profile; inspect localStorage keys. |
| AC2 | System mode follows OS theme changes. | Manual: change OS theme while app open in system mode. |
| AC3 | No incorrect theme flash on initial load/hard refresh/first launch; bootstrap sets `.dark`, `color-scheme`, and `data-theme` before paint. | Manual hard refresh/first-launch test in light and dark OS modes; inspect initial HTML state before app hydration. |
| AC4 | Appearance changes do not affect workspace dirty state, Save/Discard, or undo/redo. | Manual: change appearance, observe dirty/save/undo indicators; automated/state inspection if available. |
| AC5 | Every built-in preset has valid light/dark tokens and passes schema validation. | Automated fixture tests in `frontend/src/themes/schema.test.ts`. |
| AC6 | Generated preset CSS is complete, current, and valid for every built-in preset/mode. | Automated generated-CSS stale/validity tests; build runs generation/check. |
| AC7 | Contrast meets WCAG 2.2 AA: 4.5:1 normal text, 3:1 large text, 3:1 meaningful non-text UI components and graphical objects required to understand/use the UI, across all built-in presets/modes. | Contrast checker/tooling + manual review; document results per preset/mode. |
| AC8 | Modernized UI components have visible keyboard focus and keyboard operation; reduced motion is respected. | Manual keyboard-only pass; component tests where feasible; manual reduced-motion verification by enabling OS/browser reduced-motion setting and confirming non-essential motion is reduced/disabled. |
| AC9 | Canvas and all views readable across presets/modes. | Manual visual QA matrix. |
| AC10 | Escape cancels active native drag without moving window, including continued Ctrl+LButton hold and release/restart cases. | Pure/state-machine tests + manual Windows test; code inspection. |
| AC11 | Release outside zone does not move window. | Manual Windows test; existing code path inspection. |
| AC12 | Disabled-layout suppression unchanged. | Existing `native_loop` tests + manual Windows test. |
| AC13 | Existing Rust/frontend/embedded-asset build workflow intact. | Automated commands in WP6. |

## Verification Plan

### Automated

- Rust: `cargo fmt --check`
- Rust: `cargo clippy --all-targets -- -D warnings`
- Rust: `cargo test`
- Frontend: `cd frontend && bun run check`
- Frontend: `cd frontend && bun test`
- Frontend: `cd frontend && bun run build`

### Manual (must be reported as manual, not claimed if not run)

- Windows overlay: Ctrl+drag, hover emphasis, labels, Esc cancel (including hold-through-cancel, release, fresh gesture), release outside zone, disabled layout.
- Browser/editor: mode/preset switch, persistence, system mode follow, hard refresh/first launch no flash, per-browser-profile scope, `mode-watcher-mode`/`mode-watcher-theme` synchronization.
- Workspace integrity: appearance changes do not dirty draft or create undo history.
- Accessibility: keyboard-only, visible focus, reduced motion (enable OS/browser "Reduce motion"/"prefers-reduced-motion" and verify non-essential animations/transitions are reduced or disabled), 200% zoom, contrast (4.5:1 normal text, 3:1 large text, 3:1 meaningful non-text UI components/graphical objects).

## Accessibility Limitations (explicit)

- **Not fully accessible in first implementation**: this plan does not claim full accessibility completion for every component/view.
- **Intentionally deferred/limited**: `resizable.svelte` keyboard resize behavior may remain limited unless implemented in WP3; if deferred, it must be documented as a known limitation. Native Win32 overlay text/label rendering is visual-only and not exposed through browser accessibility APIs; keyboard zone selection during drag is deferred.
- **Required now**: modernized components in scope must meet AC7/AC8 criteria; any component not modernized must not be described as accessibility-complete.

## Risks and Mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| FOUC from bootstrap/ModeWatcher conflict | Visible wrong theme flash | Single init strategy; explicit storage keys; manual hard-refresh tests. |
| Token remap regressions | Visual inconsistency | Exact one-to-one `zf-*` mapping, alias layer, visual QA matrix. |
| Primitive replacement regressions | Broken dialogs/menus/tabs | Selective migration, adapters, keyboard/a11y tests. |
| Native overlay label rendering complexity | GDI text/layout issues | Start with simple index badge; keep lightweight; do not block safety features. |
| Contrast failures in presets | Accessibility non-compliance | Validate fixtures and run contrast checks for every preset/mode. |

## Review Checklist (adversarial)

- [ ] No claim contradicts current code.
- [ ] No theme fields added to Rust workspace config/DTO/API.
- [ ] No appearance change affects dirty/undo/save/discard.
- [ ] ModeWatcher uses `mode-watcher-mode`/`mode-watcher-theme`; bootstrap is single pre-paint owner; `ModeWatcher` uses `disableHeadScriptInjection={true}`.
- [ ] No competing theme initialization scripts; no duplicated preset token values in bootstrap.
- [ ] Generated CSS is source-derived, complete, current, and valid.
- [ ] Every `zf-*` token has exactly one semantic mapping; `zf-faint` -> `muted-foreground-faint`.
- [ ] Preset schema uses literal allowed-key types/runtime allowlists; required/optional tokens explicit; no overlay preset tokens.
- [ ] Native Escape cancel uses `cancelled_until_release` latch with required test cases; no `MoveWindow` during cancelled gesture.
- [ ] Native zone labels use internal-only payload extension; native cache is visualization/hit-test only, worker resolves final `MoveWindow` from authoritative state; no HTTP endpoint/workspace DTO/theme API/persisted field.
- [ ] WP dependencies are acyclic and independently reviewable; `index.html` changes only in WP2; WP1 owns schema/fixtures/CSS generation.
- [ ] Every acceptance criterion has a verification method; accessibility ratios/reduced-motion explicitly stated.
- [ ] Manual Windows behavior is not claimed unless actually tested.
- [ ] Out-of-scope items remain deferred.
