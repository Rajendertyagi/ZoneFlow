# Updated Plan: Overlay/UX Polish + Modern Borderless Theme System

## Changes from previous plan
- Removed **multi-monitor context label** from overlay polish (not needed).
- Added **JSON-driven theme system** based on tweakcn/shadcn registry theme JSON (example: `https://tweakcn.com/r/themes/caffeine.json`), including where theme JSON lives, format, build/load flow, and token mapping.
- Added **doc-backed stack decisions** (shadcn-svelte, Bits UI, mode-watcher, Tailwind v4) and an explicit “no handrolled UI primitives” rule.
- Applied **audit corrections** (subagent audit, session `ses_df66978ffffd3IZDWBsJqUUuYh`): clarified Vite SPA FOUC/bootstrap, resolved persistence precedence, made token architecture explicit for Tailwind v4, defined preset normalization/sanitization, clarified native overlay mapping, and added validation/test gates.

## Scope
Plan only. No project code changes made. Based on code study of `D:\Temp\ScreenLayoutRust`, official docs, and audit findings.
- shadcn-svelte Theming: https://shadcn-svelte.com/docs/theming
- shadcn-svelte Dark Mode (Svelte): https://shadcn-svelte.com/docs/dark-mode/svelte
- mode-watcher docs: https://mode-watcher.svecosystem.com/docs , https://mode-watcher.svecosystem.com/docs/components/mode-watcher , https://mode-watcher.svecosystem.com/docs/states/theme
- tweakcn/shadcn theme JSON example: https://tweakcn.com/r/themes/caffeine.json

## Current State (code study, audit-verified)

- **App shape**: headless Rust + Axum + Svelte 5 SPA embedded in exe, served at `127.0.0.1:4000`; browser-on-demand editor, native tray/hotkeys/overlay (`README.md`, `src/native_loop.rs`, `frontend/`).
- **Frontend stack**: Svelte 5, Vite 8, Tailwind 4, shadcn-svelte, Bits UI, Paneforge, Lucide, tailwind-merge (`frontend/package.json`). No `mode-watcher`.
- **shadcn-svelte configured but underused**: `frontend/components.json` exists (`style: new-york`, `baseColor: slate`, css `src/app.css`, aliases). `shadcn-svelte` is devDependency; `bits-ui` is dependency.
- **UI primitives are handrolled**: `frontend/src/lib/components/ui/` contains 14 custom files (`badge`, `button`, `context-menu`, `dialog`, `input`, `label`, `resizable`, `scroll-area`, `select`, `separator`, `slider`, `switch`, `tabs`, `tooltip`). No Bits UI imports found in `frontend/src`. Toasts are custom in `App.svelte`. `sonner` is not present. Replacement is an **API/behavior migration**, not a drop-in styling swap.
- **Current theme**: single hardcoded dark palette in `frontend/src/app.css` under `@theme` with `--color-zf-*` tokens. `zf-*` usage is widespread (audit found 311 matches across 32 files). Canvas/SVG has hardcoded hex colors. No light/dark/system mode, no preset/theme setting in config/DTO/UI.
- **Overlay**: native Win32 layered `UpdateLayeredWindow` during drag (`src/native_loop.rs:paint_overlay`). Uses `zone_tints`, `overlay_inactive_alpha`, `overlay_active_alpha`; hovered zone gets thicker opaque border. No labels, no Esc cancel hint, no snap feedback, no keyboard assist. Gesture handling is Ctrl+LButton only.
- **Settings flow**: `src/config.rs::Settings` -> `src/api/dto.rs::SettingsDto` -> `frontend/src/lib/api/client.ts` -> `SettingsView.svelte`.

## Doc-backed Decisions

| Decision | Official basis | Plan implication |
|---|---|---|
| Use semantic CSS vars (`background/foreground/primary/...`) with `:root` and `.dark` | shadcn-svelte theming docs | Define raw semantic vars in `:root/.dark`; map to Tailwind tokens via `@theme inline`; keep `zf-*` aliases during migration. |
| Use `@theme inline` / Tailwind 4 CSS-first token registration | shadcn-svelte theming docs | Use `@theme inline { --color-background: var(--background); ... }` pattern. Do not rely on old tailwind.config theme extension. |
| Use `mode-watcher` for dark mode | shadcn-svelte dark mode docs recommend `mode-watcher`; mode-watcher docs provide `ModeWatcher`, `setMode`, `toggleMode`, `mode`, `theme` | Add `mode-watcher`. In plain Svelte/Vite SPA, mount `<ModeWatcher />` in root `App.svelte` (not SvelteKit `+layout.svelte`). |
| Use mode-watcher `theme` / `data-theme` only if helpful | mode-watcher `theme` state sets `data-theme` on html | Use `mode-watcher` for **mode** (`system/light/dark`). Preset loader applies CSS vars; optionally set `data-theme="<preset-id>"` for debugging/targeted overrides. |
| Use shadcn-svelte/Bits UI primitives instead of handrolled UI | shadcn-svelte components are built on Bits UI; project already has both installed | Replace handrolled primitives with shadcn-svelte components; keep custom only for domain-specific canvas/overlay. Inventory call sites and migrate by primitive. |
| Use tweakcn/shadcn registry JSON as preset format | tweakcn export matches shadcn token names (`cssVars.theme/light/dark`) | Store presets in `frontend/src/themes/*.json`; validate/normalize/dedupe known keys; whitelist or ignore unexpected `css`. |

## Design Direction

### “Borderless UI”
The editor is a browser SPA, so “borderless” means **low-chrome, border-light visual design**, not a native frameless OS window:
- Prefer surface elevation, spacing, contrast, and subtle shadows/hairlines over heavy borders.
- Keep borders only where needed for affordance/accessibility (inputs, focus, drag handles, critical separators).
- Use layered surfaces (`background/card/popover/muted`), soft depth, clear typography, compact studio density.
- Avoid nested boxed-in sections.

### Modern theme support
- **Modes**: `system | light | dark` via `mode-watcher`.
- **Color variations**: multiple theme presets built from JSON (e.g. Caffeine, ZoneFlow Amber, Ocean). User selects preset + mode.
- **Semantic tokens**: components consume semantic roles, not raw palette values.
- **Accessibility**: contrast checks, visible `focus-visible`, non-color-only status, reduced motion, minimum hit targets.

## JSON Theme System

### Where theme JSON lives
Put built-in theme preset JSON files in frontend source so they are bundled into the embedded SPA:

```text
frontend/src/themes/
  caffeine.json          # tweakcn/shadcn registry theme JSON (example/reference)
  zoneflow-amber.json    # default ZoneFlow preset
  ocean.json             # optional color variation
  index.ts               # theme manifest/loader/validation
  types.ts               # Theme JSON types
  apply-theme.ts         # runtime apply helpers (mode/preset)
```

**Why here:** `frontend/src/` is built by Vite and embedded via `rust-embed`; no runtime file lookup, no extra backend endpoint required for static presets. User-imported custom themes (future) can be stored in config as validated JSON/string, but built-in presets live in `frontend/src/themes/`.

### Theme JSON format
Use the **tweakcn/shadcn registry style** format as canonical input, compatible with `caffeine.json`:

```json
{
  "$schema": "https://ui.shadcn.com/schema/registry-item.json",
  "name": "caffeine",
  "type": "registry:style",
  "cssVars": {
    "theme": { "radius": "...", "font-sans": "...", "shadow-*": "..." },
    "light": { "background": "oklch(...)", "primary": "...", "...": "..." },
    "dark":  { "background": "oklch(...)", "primary": "...", "...": "..." }
  },
  "css": { "@layer base": { "body": { "letter-spacing": "var(--tracking-normal)" } } }
}
```

**Rules:**
- `cssVars.light` and `cssVars.dark` required for built-in presets.
- `cssVars.theme` holds shared tokens (radius, fonts, shadows, tracking, spacing).
- Normalize/dedupe overlapping keys (tweakcn exports may duplicate font/shadow/radius/tracking across `theme` and `light/dark`).
- Optional `css` allowed only through an allowlist/sanitizer; prefer token-only presets. Built-in presets are trusted repo files; future user-imported JSON must be strictly validated.
- Keep token names shadcn-compatible for tweakcn drop-in compatibility.

### Build/load flow

1. **Authoring/import**: drop a tweakcn JSON file into `frontend/src/themes/`.
2. **Manifest**: `index.ts` imports built-in JSON presets and exports typed registry (`id`, `label`, `json`, `swatches`).
3. **Normalization**: `apply-theme.ts` validates required keys, normalizes/dedupes values, and maps to CSS variables.
4. **Application**: at startup and on change, apply selected preset + resolved mode:
   - `mode-watcher` manages `.dark` class and `color-scheme` on `<html>`.
   - preset loader applies CSS vars for active mode (`light`/`dark`) and shared `theme` vars.
   - optionally set `data-theme="<preset-id>"`.
5. **FOUC/bootstrap (Vite SPA)**: add an explicit pre-paint strategy in `index.html` (small inline script) or equivalent early bootstrap before app mount to set initial mode class/color-scheme from a local mirror. Coordinate with `ModeWatcher` so it does not fight the bootstrap. Validate no flash on reload.
6. **Persistence precedence**: **AppConfig/DTO is authoritative** for `theme_mode` and `theme_preset`. `mode-watcher` localStorage is a UX cache; on load, seed mode-watcher from config (or reconcile with defined rule); on user change, write config and update mode-watcher/local mirror. Use explicit storage keys if needed. Invalid preset/mode falls back to defaults (`zoneflow-amber`, `system`).
7. **Build-time option**: optional script (`frontend/scripts/build-themes.ts`) to generate CSS. For small preset set, runtime apply is fine.

### Token mapping strategy

Adopt **shadcn-compatible semantic tokens** as primary, map existing `zf-*` during migration:

| Existing `zf-*` | New semantic token | Notes |
|---|---|---|
| `zf-bg` | `background` | app canvas/background |
| `zf-surface` | `card` / `--surface` alias | panels/cards |
| `zf-panel` | `popover` / `--panel` alias | elevated panels/dialogs |
| `zf-raised` | `muted`/`accent` or `--surface-2` alias | hover/raised state |
| `zf-border` | `border` | subtle border/hairline |
| `zf-border-strong` | `input`/`--border-strong` alias | stronger affordance |
| `zf-text` | `foreground` | primary text |
| `zf-muted` | `muted-foreground` | secondary text |
| `zf-faint` | muted/faint alias | tertiary text; ensure contrast |
| `zf-accent` | `primary` | main action/accent |
| `zf-accent-hover` | primary hover derived/token | computed or token |
| `zf-accent-ink` | `primary-foreground` | text on accent |
| `zf-ok/warn/error` | success/warn/destructive (custom tokens) | add via `@theme inline` as needed |

**Migration rule:** define semantic vars in `:root/.dark`, map them in `@theme inline` (`--color-* : var(--*)`), add temporary `--color-zf-*` aliases mapped to semantic vars, migrate components, then remove aliases.

## Proposed Architecture

### 1) Theme config/model

Add to `Settings` / `SettingsDto` / frontend types:
- `theme_mode`: `"system" | "light" | "dark"` (default `system`)
- `theme_preset`: string preset id, e.g. `"caffeine" | "zoneflow-amber"` (default `"zoneflow-amber"`)

Files:
- `src/config.rs` — defaults, serde, migration/clamp/validation, unknown-value fallback.
- `src/api/dto.rs` — DTO fields + round-trip tests.
- `frontend/src/lib/api/client.ts` — TS types.
- `frontend/src/lib/state/*` — theme store/controller, config sync.
- `frontend/src/App.svelte` — mount `<ModeWatcher />`.
- `frontend/src/main.ts` / `index.html` — pre-paint bootstrap coordination.
- `frontend/src/lib/views/SettingsView.svelte` — Appearance section.

### 2) Replace handrolled UI primitives with shadcn-svelte/Bits UI

Inventory and migrate by primitive (API/behavior migration):
- Existing handrolled: `badge`, `button`, `context-menu`, `dialog`, `input`, `label`, `resizable`, `scroll-area`, `select`, `separator`, `slider`, `switch`, `tabs`, `tooltip`.
- Add as needed: `popover`, `dropdown-menu`, `sheet`, `alert-dialog/confirm`, `sonner` (optional, only if it simplifies toasts).
- Use shadcn-svelte component patterns and Bits UI behavior; style via semantic tokens.
- Keep domain-specific components custom: `ZoneCanvas`, `ZoneRect`, `SnapGuides`, `DragPreview`, overlay/native behavior.

### 3) Native overlay theming

Win32 overlay cannot read CSS. It receives effective colors/alpha through settings/native messages:
- Keep `overlay_inactive_alpha`, `overlay_active_alpha`, `zone_tints`.
- Define explicit mapping from selected preset to overlay values (e.g. derive `zone_tints`/active border from preset primary/accent/chart palette, or keep user-configurable tints with a "use theme preset" option). Do **not** imply CSS vars are readable by Win32 overlay.
- When theme preset changes, apply settings/update native overlay style via existing `SetOverlayStyle`/settings flow.

## Overlay/UX Polish Plan

### A. Native drag-snap overlay (`src/native_loop.rs`, `src/engine_core.rs`)

| Item | Change | Notes |
|---|---|---|
| Zone labels/numbers | Draw zone index/name in overlay | Start with index badge/high-contrast label; GDI text simplest, DWrite nicer but heavier. |
| Clear active target | Stronger active fill/border, dim inactive zones | Extend existing active/inactive alpha with clearer border/label emphasis. |
| Cancel/commit affordance | `Esc` cancels drag-snap; release outside zones cancels; optional hint | Ensure no snap command sent on cancel; add subtle hint if feasible. |
| Snap feedback | Brief confirmation after successful snap | Prefer subtle non-noisy feedback; optional editor toast or transient native indicator. |
| Keyboard assist while dragging | Number/arrow keys select zone during drag | Phase after basic overlay polish; careful input capture. |

### B. Editor UX polish (`frontend/src/lib/...`)

| Item | Change | Files/areas |
|---|---|---|
| Zone editing ergonomics | Inline rename, duplicate zone/profile, delete confirmation/undo toast, resize size readout | `ZoneInspector.svelte`, `ProfilesPanel.svelte`, `ZoneCanvas.svelte`, `ui.svelte.ts` |
| Empty/loading/error states | Clear CTAs and actionable errors | `HomeView`, `EditorView`, `AppLibrary`, `SettingsView` |
| App picker quality | Icon/source/resolved path/missing badge/recent picks | `PickerDialog.svelte`, `AppLibrary.svelte`, app DTOs if needed |
| Profile management | Duplicate/reorder/quick switch/applied vs selected clarity | `ProfilesPanel.svelte`, `TopBar.svelte`, profile commands |
| Hotkeys UX | Conflict detection, reserved-by-Windows clarity, record shortcut by keypress | `HotkeysView.svelte`, hotkey helpers, backend validation |
| Undo visibility | Toast/history count/disabled reason | canvas commands, UI store, status |
| Import/export/diagnostics | Profile JSON export/import with validation; copy diagnostics | dialogs/actions, API/client-side with validation |

### C. Visual polish / borderless pass

- Replace hardcoded SVG colors in `ZoneCanvas/ZoneRect/SnapGuides/DragPreview` with theme CSS vars.
- Add elevation/shadow tokens; reduce panel borders; use surface contrast.
- Add motion tokens and `prefers-reduced-motion` handling.
- Improve typography scale/spacing rhythm; keep studio density.
- Add theme preset preview swatches in Settings.

## Phased Implementation Plan

### Phase 0 — Audit & token spec (0.5-1 day)
- Inventory `zf-*` usages and hardcoded colors.
- Inventory handrolled UI primitives and map to shadcn-svelte components/call sites.
- Define semantic token map and `zf-*` alias compatibility.
- Confirm tweakcn/shadcn JSON format and preset manifest/normalization rules.
- Decide runtime apply vs generated CSS; define mode/preset persistence precedence and FOUC bootstrap.

**Deliverable**: token map + component replacement map + theme JSON manifest + migration checklist.

### Phase 1 — Theme foundation (1-2 days)
- Add `mode-watcher`; mount `<ModeWatcher />` in `App.svelte`.
- Add `frontend/src/themes/` with `caffeine.json` (reference) and default ZoneFlow preset JSON.
- Implement theme types/manifest/loader/apply helpers with normalization/fallback.
- Add `theme_mode` and `theme_preset` to config/DTO/frontend types; defaults/migration/validation/tests.
- Refactor `app.css` to `:root/.dark` semantic vars + `@theme inline` mappings; keep `zf-*` aliases.
- Add pre-paint bootstrap coordination to avoid FOUC.

**Acceptance**: preset + mode switch updates UI instantly; persists; reload has no flash; invalid preset/mode falls back safely.

### Phase 2 — shadcn-svelte primitive migration + borderless pass (1-2 days)
- Replace handrolled primitives with shadcn-svelte/Bits UI components by inventory.
- Apply borderless style: low border, surface contrast, subtle shadow, clear focus ring.
- Add focus-visible/disabled/reduced-motion consistency.

**Acceptance**: no handrolled behavior for standard primitives; keyboard/focus/accessibility improved; contrast pass.

### Phase 3 — Canvas/editor visual refresh (1-2 days)
- Theme-aware canvas grid, zone rects, snap guides, drag preview, empty states.
- Improve zone readability and borderless panel layout across views.
- Add Appearance settings: mode, preset, preview swatches.

**Acceptance**: canvas readable in all preset/mode combinations; no hardcoded SVG hex except documented exceptions.

### Phase 4 — Overlay UX polish (1.5-2.5 days)
- **4A**: Esc cancel, release-outside cancel, clearer active/inactive styling, optional zone index/label.
- **4B**: snap feedback, hover target clarity, optional hint.
- **4C**: keyboard assist while dragging (after 4A/4B stable).

**Acceptance**: drag-snap cancel/target behavior reliable; no regression in disabled-layout overlay suppression.

### Phase 5 — Editor workflow polish (2-3 days)
- Hotkey record/conflict UX.
- App picker metadata/recent/missing badges.
- Profile duplicate/import/export.
- Undo/redo toasts and diagnostics copy.

**Acceptance**: common workflows require fewer clicks; errors actionable; state/helper tests added.

### Phase 6 — QA, accessibility, docs (1 day)
- Visual QA for light/dark + all built-in presets.
- A11y pass: keyboard nav, focus, contrast, reduced motion, zoom.
- Update README/docs/screenshots; document theme JSON location/format and how to add presets.
- Tests: config migration/DTO round-trip for theme fields, preset-id validation/fallback, mode-watcher/config sync, FOUC/manual reload check, overlay gesture decisions, frontend state/component tests.

## Verification Plan

- **Rust**: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`.
- **Frontend**: `cd frontend; bun run check; bun test; bun run build`.
- **Manual**: theme preset/mode switch, persistence, no FOUC, light/dark contrast, drag-snap cancel/hover/snap, hotkey recording, profile import/export, disabled-layout overlay suppression.
- **Accessibility**: keyboard-only, focus-visible, reduced motion, 200% zoom; contrast check for every built-in preset in light/dark.

## Risks / Decisions to Lock

- **Token compatibility**: adopting shadcn token names eases tweakcn JSON import but requires `zf-*` alias migration. Recommended.
- **Persistence precedence**: AppConfig/DTO authoritative; mode-watcher/localStorage used as UX cache/bootstrap mirror with explicit sync rules.
- **Plain Svelte/Vite integration**: mount `ModeWatcher` in `App.svelte`; add SPA pre-paint bootstrap to avoid FOUC.
- **Overlay text rendering**: GDI text simplest; DWrite better but heavier. Start simple.
- **JSON safety**: built-in presets trusted repo files; user-imported JSON (future) must be validated/sanitized (allowed keys/values, no arbitrary CSS injection).
- **Borderless vs affordance**: keep subtle strokes/focus rings; do not remove all affordances.

## Suggested first implementation slice

**Phase 1 + Phase 2 + Phase 4A**: add `mode-watcher`, JSON theme presets (`caffeine` + default ZoneFlow preset), `system/light/dark`, semantic token foundation, shadcn-svelte/Bits UI primitive migration for core UI, borderless styling, and overlay Esc-cancel/active-target polish. This delivers visible modernization with controlled scope.