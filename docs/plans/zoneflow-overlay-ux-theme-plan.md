# ZoneFlow Overlay UX + Theme System — Implementation-Ready Plan

## Status

- **Plan type**: documentation/specification only. No application feature implementation in this task.
- **Repository**: `Rajendertyagi/ZoneFlow`
- **Baseline commit verified**: `7e55c39` (`fix: correct autostart README and delayed-window test timing`, `origin/main` HEAD at audit time)
- **Plan file**: `docs/plans/zoneflow-overlay-ux-theme-plan.md`
- **Supersedes**: prior overlay/theme plan content; this revision locks the decisions and scope below.

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
5. **Initialization**: one early initialization strategy; light/dark/system and selected preset are available before UI is painted; no competing initialization scripts; hard refresh and first launch must not show incorrect theme flash.
6. **Theme JSON**: exact typed JSON schema and allowed token keys; built-in presets only in first release; no arbitrary user CSS and no custom theme import; normalize `cssVars.theme`, `cssVars.light`, `cssVars.dark`; validate actual preset fixtures, not only pseudocode; define safe fallback for missing/invalid preset IDs.
7. **Design tokens**: inventory every existing `zf-*` token and hardcoded color; define one exact semantic mapping for each existing token; no ambiguous mappings; define foreground/background pairs, focus, border, destructive, success, warning, and canvas-specific tokens; keep existing aliases until migration is verified complete.
8. **UI primitives**: do not replace every custom component. Provide component-by-component inventory with current behavior, accessibility gaps, recommended action, and acceptance criteria. Preserve native HTML controls/simple wrappers where appropriate. Evaluate dialog, context menu, tabs, tooltip, resizable, switch individually. Do not blindly overwrite current `.svelte` components with generated shadcn-svelte files. Preserve currently installed compatible dependency versions unless a specific upgrade is justified and documented.
9. **Native overlay**: document existing non-activating, click-through behavior; specify Escape polling during active gesture using existing native loop; Escape must cancel without sending window-move command; releasing outside zones must not send move command; preserve disabled-layout suppression; define exactly how active target emphasis and native zone labels work; do not promise snap-success feedback unless confirmed worker results can reach native UI; defer keyboard zone selection until separately specified; keep overlay color settings independent of editor theme preferences in first implementation.
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
| `--color-zf-faint` | `#64748b` | `--zf-faint` (custom alias) mapped to a validated tertiary text token derived from `--muted-foreground`/theme | Keep as alias until migration complete; must pass contrast for its intended uses. |
| `--color-zf-accent` | `#f59e0b` | `--primary` | Primary action/accent. |
| `--color-zf-accent-hover` | `#fbbf24` | `--primary-hover` (custom semantic token) | Explicit token; no ad-hoc hover color. |
| `--color-zf-accent-ink` | `#1a1204` | `--primary-foreground` | Text/icon on primary. |
| `--color-zf-ok` | `#34d399` | `--success` | Success/status token. |
| `--color-zf-warn` | `#fbbf24` | `--warning` | Warning/status token. |
| `--color-zf-error` | `#f87171` | `--destructive` | Destructive/error token. |

### Required semantic token set (minimum)

Define in `:root` and `.dark`, register through Tailwind v4 `@theme inline`, and consume via semantic utilities/classes:

- Surfaces/text: `background`, `foreground`, `card`, `card-foreground`, `popover`, `popover-foreground`, `secondary`, `secondary-foreground`, `muted`, `muted-foreground`.
- Actions/focus: `primary`, `primary-hover`, `primary-foreground`, `ring`.
- Borders/inputs: `border`, `input`.
- Status: `destructive`, `destructive-foreground`, `success`, `success-foreground`, `warning`, `warning-foreground`.
- Canvas-specific: `canvas-bg`, `canvas-grid`, `canvas-zone-stroke`, `canvas-zone-selected`, `canvas-zone-drop`, `canvas-zone-merge`, `canvas-handle-shared`, `canvas-handle-outer`, `canvas-preview`, `canvas-label-bg`, `canvas-label-fg`.
- Overlay-mapped semantic source tokens (editor/CSS only): `overlay-tint-*` or preset swatch tokens used for documentation/preview; native overlay continues to use existing settings fields.

No component may map a `zf-*` token to multiple semantic targets. Compatibility aliases may point old `zf-*` names to the new semantic tokens until removal.

### Hardcoded color inventory (must be tokenized or explicitly justified)

| Location | Current hardcoded colors | Action |
|---|---|---|
| `frontend/src/app.css` | `#070b14`, `#0d1526`, `#111c31`, `#17243d`, `#22304a`, `#33445f`, `#e6edf7`, `#93a4bd`, `#64748b`, `#f59e0b`, `#fbbf24`, `#1a1204`, `#34d399`, `#fbbf24`, `#f87171` | Replace with semantic vars + preset values; keep aliases during migration. |
| `frontend/src/lib/views/SettingsView.svelte` | default `zone_tints` hex array | Keep as overlay settings defaults; independent from editor theme preset. |
| `frontend/src/lib/components/Canvas/ZoneCanvas.svelte` | grid stroke `#22304a`, empty-state text `#93a4bd/#6b7c96`, fallback tint `#f59e0b` | Replace with canvas semantic tokens/CSS vars. |
| `frontend/src/lib/components/Canvas/ZoneRect.svelte` | strokes/fills `#f59e0b`, `#c792ea`, `#33445f`, badge bg `#0b1322`, text `#e6edf7/#93a4bd/#1a1204`, handles `#6EA8FE/#f59e0b` | Replace with canvas semantic tokens; preserve meaning of selected/drop/merge/shared/outer handles. |
| `frontend/src/lib/components/Canvas/DragPreview.svelte` | preview fill/stroke/text `#f59e0b/#fbbf24` | Replace with `canvas-preview` tokens. |
| `frontend/src/lib/components/Canvas/SnapGuides.svelte` | guide/hud `#f59e0b/#0b1322/#fbbf24` | Replace with canvas/overlay-guide tokens. |
| `frontend/src/lib/components/ui/*` | `bg-white`, `bg-black/70` in dialog/switch as applicable | Evaluate: keep only if mapped to semantic overlay/switch-thumb tokens or documented exception. |

## Theme System Design

### Mode and preset ownership

- **Mode**: `system | light | dark`, managed by `mode-watcher`.
- **Preset**: built-in preset ID string, managed by app theme controller and persisted by ModeWatcher/localStorage mechanism (explicit key names documented in WP1/WP2).
- **Authoritative source for appearance**: browser-local ModeWatcher/preferences state, not workspace config.
- **Workspace config**: unchanged for theme mode/preset.

### File layout

```text
frontend/src/themes/
  schema.ts            # TypeScript types + runtime guards/normalization helpers
  presets/
    zoneflow-amber.json
    caffeine.json
  index.ts             # built-in manifest, default/fallback, lookup
  apply.ts             # apply normalized preset vars to documentElement
  apply.test.ts        # fixture validation/normalization/fallback tests
frontend/src/lib/theme/
  preference.ts        # ModeWatcher/localStorage keys, load/save/fallback
  preference.test.ts
frontend/index.html     # single pre-paint bootstrap script (mode + preset id + minimal critical vars/class)
frontend/src/main.ts     # imports app.css; no competing theme init
frontend/src/App.svelte  # mounts ModeWatcher once
```

Built-in preset JSON files are bundled by Vite and embedded by existing `rust-embed` pipeline; no runtime file lookup and no Rust endpoint.

### Exact theme JSON schema (built-in presets)

```ts
type ThemePreset = {
  $schema?: string; // optional metadata, ignored by runtime after validation
  name: string;     // preset id, kebab-case, stable
  label: string;    // UI label
  type: 'registry:style';
  cssVars: {
    theme: Record<string, string>; // shared non-color tokens
    light: Record<string, string>; // light semantic tokens
    dark: Record<string, string>;  // dark semantic tokens
  };
};
```

### Allowed token keys

`cssVars.theme` allowed keys:
- `radius`, `font-sans`, `font-mono`, `shadow-*` (explicitly enumerated), `tracking-*` (explicitly enumerated), `spacing` (if used).

`cssVars.light` and `cssVars.dark` allowed keys (exact set):
- `background`, `foreground`, `card`, `card-foreground`, `popover`, `popover-foreground`, `primary`, `primary-hover`, `primary-foreground`, `secondary`, `secondary-foreground`, `muted`, `muted-foreground`, `accent`, `accent-foreground`, `destructive`, `destructive-foreground`, `success`, `success-foreground`, `warning`, `warning-foreground`, `border`, `input`, `ring`, `chart-1`, `chart-2`, `chart-3`, `chart-4`, `chart-5`, `canvas-bg`, `canvas-grid`, `canvas-zone-stroke`, `canvas-zone-selected`, `canvas-zone-drop`, `canvas-zone-merge`, `canvas-handle-shared`, `canvas-handle-outer`, `canvas-preview`, `canvas-label-bg`, `canvas-label-fg`.

Rules:
- Unknown keys are ignored with dev warning and omitted from applied output; they do not fail safe fallback unless required keys are missing.
- Missing required semantic keys cause preset validation failure for that preset; loader falls back to default `zoneflow-amber`.
- `css` blocks from tweakcn exports are **not applied** in first implementation. Built-in fixtures may contain no `css`; if present in a future imported file, it is rejected. First release has no user import.
- Values are validated as non-empty strings; color tokens must parse as CSS color/supported format used by project (prefer hex/rgb/hsl/oklch as authored). Invalid value => preset invalid.

### Normalization

- Deduplicate keys across `theme/light/dark`: shared tokens live in `theme`; mode tokens live in `light/dark`.
- If a key appears in both `theme` and `light/dark`, mode-specific value wins for that mode and normalization records a dev warning.
- Trim strings; preserve authored color format; do not silently coerce unknown formats.
- Produce a normalized in-memory object before applying.

### Fallback behavior

- Missing/invalid stored preset ID => use `zoneflow-amber`.
- Missing/invalid stored mode => ModeWatcher/default `system`.
- Invalid preset fixture at build/test time => test failure; runtime uses default preset.
- If localStorage is unavailable/corrupt => run with defaults, no crash.

### Pre-paint initialization (single strategy)

- `frontend/index.html` contains one small inline script before module script that:
  - reads only appearance keys (mode, preset id) from localStorage;
  - validates against built-in preset id allowlist embedded as a tiny constant/data attribute or minimal inline map;
  - sets initial `class="dark"`/`color-scheme` as needed;
  - applies minimal critical CSS variables for initial background/foreground/surfaces (or a pre-generated critical subset) to prevent flash.
- `ModeWatcher` is mounted once in `App.svelte` with explicit storage keys and configured to not fight bootstrap.
- `main.ts` does not initialize theme. No other script mutates theme before paint.
- Hard refresh and first launch verification required in WP2/WP6.

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

### Planned behavior

| Behavior | Specification | Verification |
|---|---|---|
| Escape cancel | During active gesture, poll `VK_ESCAPE` in native loop. If pressed while active, hide overlay, clear active drag state, and do **not** send `MoveWindow`. Releasing Ctrl/LButton after cancel must not snap. | Manual Windows test; add/extend pure gesture-state tests where possible. |
| Release outside zone | Keep current behavior: no `MoveWindow` command sent. | Code inspection + manual Windows test. |
| Disabled layout suppression | Keep current `gesture_step` behavior; no overlay shown while disabled; in-flight drag cancels without drop. | Existing tests + manual Windows test. |
| Active target emphasis | Hovered zone gets stronger border/alpha/label emphasis; inactive zones dimmed. Use existing alpha settings and tint palette; no theme-preset dependency. | Manual Windows visual test. |
| Zone labels | Draw zone index badge/label in overlay. First implementation uses GDI text (`DrawTextW`/`TextOutW`) or equivalent simple GDI path; keep implementation lightweight. Label content: zone index (1-based) and optional short label if available in cached zone data; if adding label requires payload change, include it explicitly in WP5. | Manual Windows visual test. |
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

- **Scope**: Introduce semantic CSS variables, Tailwind v4 `@theme inline` mappings, `zf-*` compatibility aliases, canvas semantic tokens, and preset JSON schema/fixtures.
- **Non-goals**: No ModeWatcher UI, no Rust config/DTO changes, no user import/CSS.
- **Dependencies**: WP0.
- **Exact files/areas**: `frontend/src/app.css`, `frontend/src/themes/schema.ts`, `frontend/src/themes/presets/*.json`, `frontend/src/themes/index.ts`, `frontend/src/themes/apply.ts`, `frontend/src/themes/*.test.ts`, `frontend/index.html` (only if needed for critical token bootstrap coordination), `frontend/src/main.ts` (no competing init).
- **Data flow/state ownership**: Built-in preset JSON -> normalize/validate -> apply CSS vars to `documentElement`. No workspace draft/state.
- **Behavior**: Default preset is `zoneflow-amber`; invalid preset falls back to default; semantic tokens cover all existing `zf-*` mappings.
- **Failure/fallback**: Invalid token/key/value => preset rejected/fallback; unknown keys ignored with dev warning.
- **Automated tests**: Fixture validation tests for every built-in preset; normalization/dedupe/fallback unit tests.
- **Manual acceptance**: App renders with default theme; no missing token regressions in light/dark baseline.
- **Completion criteria**: All `zf-*` tokens mapped exactly once; aliases present; preset fixtures pass tests.
- **Regression risks**: Visual regressions from token remap; mitigate by alias layer and visual QA.

### WP2 — Theme preferences and JSON presets

- **Scope**: Add ModeWatcher, appearance controls, local preference persistence, pre-paint bootstrap, preset switching.
- **Non-goals**: No workspace Settings/DTO/API changes; no dirty/undo/history; no custom import.
- **Dependencies**: WP1.
- **Exact files/areas**: `frontend/package.json` (add `mode-watcher`, pinned compatible version; no unrelated dependency upgrades), `frontend/src/App.svelte`, `frontend/index.html`, `frontend/src/main.ts`, `frontend/src/lib/theme/preference.ts`, `frontend/src/lib/views/SettingsView.svelte` or dedicated appearance component, `frontend/src/lib/components/*` as needed for appearance UI.
- **Data flow/state ownership**: ModeWatcher/localStorage owns mode/preset. UI reads/writes appearance prefs only; does not call settings save endpoints for appearance.
- **Behavior**: Mode and preset apply instantly; survive reload/restart; system mode follows OS; no FOUC; per-browser-profile scope stated in UI. If appearance controls live in `SettingsView`, they must be isolated from the workspace settings draft/save flow and must not trigger `setSettings`/`setAppOptions`/`save`/undo history.
- **Failure/fallback**: localStorage unavailable/corrupt => defaults; invalid stored values => defaults.
- **Automated tests**: preference parse/fallback tests; component/state tests for appearance controls where feasible.
- **Manual acceptance**: Hard refresh, first launch, system mode change, preset switch, restart persistence; workspace dirty/save/undo unaffected.
- **Completion criteria**: All WP2 acceptance criteria pass; no Rust API/config changes.
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
- **Manual acceptance**: Keyboard-only navigation, focus-visible, Escape/Tab behavior, screen-reader labels/roles for modernized primitives.
- **Completion criteria**: Inventory table actions completed or explicitly deferred with justification; no behavior regressions.
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

- **Scope**: Escape cancel, active-target emphasis, zone labels; preserve existing overlay safety behavior.
- **Non-goals**: No keyboard zone selection; no snap-success feedback unless worker confirmation path is implemented; no theme-preset coupling.
- **Dependencies**: WP0; independent of WP2-WP4 except shared release QA.
- **Exact files/areas**: `src/native_loop.rs`, possibly `src/api/commands.rs`/`src/api/state.rs`/native message payloads only if required for zone labels (not a theme-preference API); tests in `src/native_loop.rs`.
- **Data flow/state ownership**: Native main thread owns overlay/gesture state; worker remains authoritative for move command execution. Escape cancel prevents sending `MoveWindow`.
- **Behavior**: Esc cancels active drag without move; release outside zone no move; disabled layout suppresses/cancels; hovered zone clearly emphasized; labels visible and non-intrusive.
- **Failure/fallback**: If label rendering is unreliable, ship index badge/minimal label with documented limitation; do not block Escape/cancel safety.
- **Automated tests**: Extend pure gesture/state tests; add tests for cancel state transitions where possible.
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
| AC1 | Mode (`light/dark/system`) and preset survive reload/restart in same browser profile. | Manual browser test: set values, reload, restart app/browser profile. |
| AC2 | System mode follows OS theme changes. | Manual: change OS theme while app open in system mode. |
| AC3 | No incorrect theme flash on initial load/hard refresh/first launch. | Manual hard refresh/first-launch test in light and dark OS modes. |
| AC4 | Appearance changes do not affect workspace dirty state, Save/Discard, or undo/redo. | Manual: change appearance, observe dirty/save/undo indicators; automated/state inspection if available. |
| AC5 | Every built-in preset has valid light/dark tokens and passes schema validation. | Automated fixture tests in `frontend/src/themes/*.test.ts`. |
| AC6 | WCAG 2.2 AA contrast checks pass for text/UI/status/canvas affordances in all built-in presets/modes. | Contrast checker/tooling + manual review; document results. |
| AC7 | Keyboard navigation and visible focus work for modernized UI primitives. | Manual keyboard-only pass; component tests where feasible. |
| AC8 | Canvas and all views readable across presets/modes. | Manual visual QA matrix. |
| AC9 | Escape cancels active native drag without moving window. | Manual Windows test; code inspection of cancel path. |
| AC10 | Release outside zone does not move window. | Manual Windows test; existing code path inspection. |
| AC11 | Disabled-layout suppression unchanged. | Existing `native_loop` tests + manual Windows test. |
| AC12 | Existing Rust/frontend/embedded-asset build workflow intact. | Automated commands in WP6. |

## Verification Plan

### Automated

- Rust: `cargo fmt --check`
- Rust: `cargo clippy --all-targets -- -D warnings`
- Rust: `cargo test`
- Frontend: `cd frontend && bun run check`
- Frontend: `cd frontend && bun test`
- Frontend: `cd frontend && bun run build`

### Manual (must be reported as manual, not claimed if not run)

- Windows overlay: Ctrl+drag, hover emphasis, labels, Esc cancel, release outside zone, disabled layout.
- Browser/editor: mode/preset switch, persistence, system mode follow, hard refresh/first launch no flash, per-browser-profile scope.
- Workspace integrity: appearance changes do not dirty draft or create undo history.
- Accessibility: keyboard-only, focus-visible, reduced motion, 200% zoom, contrast.

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
- [ ] No competing theme initialization scripts.
- [ ] Every `zf-*` token has exactly one semantic mapping.
- [ ] Preset schema/normalization/fallback is explicit and testable.
- [ ] WP dependencies are acyclic and independently reviewable.
- [ ] Every acceptance criterion has a verification method.
- [ ] Manual Windows behavior is not claimed unless actually tested.
- [ ] Out-of-scope items remain deferred.
