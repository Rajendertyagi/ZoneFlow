# Theme, presets and appearance preferences

This document describes the editor theme system delivered by WP1–WP4 of the
[ZoneFlow Overlay UX + Theme plan](plans/zoneflow-overlay-ux-theme-plan.md) and
the low-chrome visual polish pass delivered by VWP0–VWP5 of the
[ZoneFlow Visual Polish plan](plans/zoneflow-visual-polish-plan.md). It is
documentation only — it does not change application behaviour. Bun is a
**build-time tool only**; the installed app has no Node/Bun runtime.

For the automated WCAG 2.2 AA contrast calculation recorded in VWP6, see
[visual-contrast-report.md](visual-contrast-report.md). That report is an
automated calculation, **not** visual verification; the manual visual pass
remains NOT YET PERFORMED in
[manual-verification-checklist.md](manual-verification-checklist.md).

## Where built-in presets live

Built-in theme presets are JSON files in `frontend/src/themes/presets/`:

- `zoneflow-amber.json` — the default and fallback preset.
- `caffeine.json` — the second built-in preset.

These JSON fixtures are the **single source of truth** for token values. Complete
static CSS for every preset and both modes (`light`/`dark`) is generated from them
into `frontend/src/themes/generated/presets.css` during the frontend build. The
pre-paint bootstrap in `frontend/index.html` contains only preset **IDs** — it
never duplicates token values.

Related files:

| File | Role |
|---|---|
| `frontend/src/themes/schema.ts` | Literal allowed-key types, runtime allowlists, normalization and validation. |
| `frontend/src/themes/manifest.ts` | Built-in manifest metadata (`id`, human-readable `label`, picker `swatches`), default/fallback ID and lookup helpers. |
| `frontend/src/themes/generate-css.ts` | Build script: JSON fixtures → generated static CSS. |
| `frontend/src/themes/generated/presets.css` | Generated output (committed; checked for staleness). |
| `frontend/src/themes/schema.test.ts` | Fixture validation / normalization / fallback tests. |
| `frontend/src/themes/generated-css.test.ts` | Generated-CSS current/valid tests. |
| `frontend/src/lib/theme/preference.ts` | ModeWatcher storage-key constants and pure parse/fallback helpers. |
| `frontend/index.html` | Single pre-paint initialization owner. |
| `frontend/src/App.svelte` | Mounts `ModeWatcher` once with `disableHeadScriptInjection={true}`. |
| `frontend/src/lib/components/AppearanceSettings.svelte` | Appearance controls in the Settings view. |

## Preset JSON format (tweakcn-compatible)

Each preset file uses the registry-style shape used by tweakcn/shadcn theme JSON:

```json
{
  "name": "zoneflow-amber",
  "type": "registry:style",
  "cssVars": {
    "theme": { "radius": "0.5rem", "font-sans": "...", "font-mono": "..." },
    "light": { "background": "#f7f8fb", "foreground": "#111827", "...": "..." },
    "dark":  { "background": "#070b14", "foreground": "#e6edf7", "...": "..." }
  }
}
```

- `name` — stable preset ID in kebab-case; used as the `data-theme` value and as
  the stored ModeWatcher theme value. It must match the manifest ID.
- `type` — always `"registry:style"`.
- `cssVars.theme` — shared keys: `radius`, `font-sans`, `font-mono` (all required).
- `cssVars.light` / `cssVars.dark` — per-mode token maps. Required keys include
  surfaces/text (`background`, `foreground`, `card`, `card-foreground`, `popover`,
  `popover-foreground`, `secondary`, `secondary-foreground`, `muted`,
  `muted-foreground`, `muted-foreground-faint`), actions/focus (`primary`,
  `primary-hover`, `primary-foreground`, `accent`, `accent-foreground`, `ring`),
  borders (`border`, `input`), status (`destructive`, `destructive-foreground`,
  `success`, `success-foreground`, `warning`, `warning-foreground`), the dialog
  scrim (`scrim`) and the canvas-specific tokens (`canvas-bg`, `canvas-grid`,
  `canvas-zone-stroke`, `canvas-zone-selected`, `canvas-zone-drop`,
  `canvas-zone-merge`, `canvas-handle-shared`, `canvas-handle-outer`,
  `canvas-preview`, `canvas-label-bg`, `canvas-label-fg`). `chart-1`…`chart-5`
  are optional.
- ZoneFlow-only extensions to the base shadcn/tweakcn set are `primary-hover`,
  `success`/`success-foreground`, `warning`/`warning-foreground` and the
  `canvas-*` tokens. They are part of the schema, fixtures, generated CSS and tests.
- Unknown keys are ignored with a dev warning; a missing required key or an
  invalid value makes the fixture invalid (build/test failure). `css` blocks are
  rejected. There are no overlay-specific preset tokens — see
  [Overlay independence](#overlay-independence).

Human-readable `label` and picker `swatches` are **not** in the JSON fixtures;
they are ZoneFlow-only presentation metadata in `manifest.ts`, so the fixtures
stay tweakcn-compatible in shape.

## Adding a built-in preset

1. Add a new tweakcn-compatible JSON file to `frontend/src/themes/presets/`
   (copy `zoneflow-amber.json` as a template and fill in **every** required token
   for both `light` and `dark`).
2. Register it in `frontend/src/themes/manifest.ts`: import the fixture and add an
   `entry('<preset-id>', '<Human label>', [<swatch>, ...], fixture)` to
   `PRESET_MANIFEST`. The fixture `name` must equal the manifest ID.
3. Add the preset ID to the inline `PRESET_IDS` allowlist in the pre-paint
   bootstrap in `frontend/index.html` (the bootstrap mirrors the manifest
   allowlist so an invalid stored value cannot be applied before paint).
4. Regenerate the static CSS and verify it is current:

   ```powershell
   cd frontend
   bun run themes:generate   # rewrite src/themes/generated/presets.css
   bun run themes:check      # fail if generated CSS is stale
   bun test                  # fixture + generated-CSS tests
   bun run check
   bun run build
   ```

5. Commit the new JSON, the manifest/bootstrap edits and the regenerated
   `presets.css` together. `bun run build` runs `themes:generate` automatically,
   and `themes:check` fails CI if the committed CSS is stale.

Invalid or unknown preset IDs always fall back to `zoneflow-amber` (both at
runtime and in the bootstrap).

## Visual direction and surface hierarchy (VWP0–VWP5)

The VWP0–VWP5 visual polish pass applies a **polished, modern, LOW-CHROME**
look without changing behaviour, workflow, or the frozen architecture
decisions. "Low-chrome" removes *decorative* boxed chrome — the repeated
`rounded-lg border border-zf-border bg-zf-surface` wrapper on every panel,
card, chip and row — and replaces it with deliberate surface-contrast steps and
subtle elevation shadows. It does **not** remove functional boundaries: input
and select borders, focus rings, drag/resize handles, meaningful separators,
status distinctions, and canvas zone strokes/handles/previews/guides are all
preserved.

Four surface levels are used, adjacent levels being distinguishable without a
border:

1. **App background** — `--background` (`bg-background` / `bg-zf-bg`).
2. **Panel surface** — `--card` (`bg-card` / `bg-zf-surface`) with `--zf-shadow-1`.
3. **Raised / interactive surface** — `--secondary` (`bg-secondary` / `bg-zf-raised`).
4. **Overlay surface** — `--popover` (`bg-popover` / `bg-zf-panel`) with `--zf-shadow-3`.

### The `scrim` token (new, required)

`scrim` was added to `REQUIRED_LIGHT_DARK_KEYS` in `schema.ts` and to both
modes of both preset JSON fixtures. It is mapped in `app.css` with
`--color-scrim: var(--scrim);` inside the `@theme inline` block, which makes
the `bg-scrim` Tailwind utility resolvable. `bg-scrim` is the **canonical**
form used by all dialogs; `bg-[var(--scrim)]` arbitrary-value syntax is not
used, and no `zf-*` alias exists for scrim. `var(--scrim)` is reserved for
contexts where a Tailwind utility cannot be applied. The previous
`bg-black/70` scrim and its stale exception comment were removed.

Current values:

| Preset | Light | Dark |
|---|---|---|
| `zoneflow-amber` | `rgba(7, 11, 20, 0.55)` | `rgba(0, 0, 0, 0.72)` |
| `caffeine` | `rgba(41, 33, 26, 0.58)` | `rgba(0, 0, 0, 0.72)` |

Dialog text sits on the opaque `popover` surface above the scrim, so dialog
text legibility is governed by the `popover-foreground` / `popover` ratio (see
the contrast report), not by the scrim value itself. Whether the scrim reads
well in each preset/mode is a **visual** judgement and remains a manual item.

### Elevation and hairline tokens (`app.css` only)

Elevation is defined **only in `frontend/src/app.css`**, not in the preset JSON
schema. This keeps shadows structural rather than per-preset identity and
avoids a second source of truth. The shadow base is **dark in both modes** and
is deliberately **not** derived from `--foreground` (which is near-white in
dark mode and would produce light glows instead of natural shadows):

| Token | Definition | Role |
|---|---|---|
| `--zf-shadow-color` | `:root { rgb(15 23 42 / 0.16) }` · `:root.dark { rgb(0 0 0 / 0.55) }` | Dark shadow base, mode-specific strength |
| `--zf-shadow-1` | `0 1px 2px var(--zf-shadow-color)` | Resting elevation for panels/cards |
| `--zf-shadow-2` | `0 4px 14px var(--zf-shadow-color)` | Hover / raised elevation |
| `--zf-shadow-3` | `0 18px 48px var(--zf-shadow-color)` | Dialogs / menus / tooltips |
| `--zf-hairline` | derived from `--foreground` via `color-mix` | Low-chrome visual separators (a light hairline in dark mode is intentional edge definition, **not** a shadow) |

### Preset identities

The two built-in presets are now meaningfully, but conservatively, distinct —
in accent hue, surface temperature **and** canvas tokens, not just a button
colour:

- **`zoneflow-amber`** (default/fallback) — amber / deep-slate identity. Dark
  background near `#070b14`, amber primary near `#f59e0b`, radius `0.5rem`.
  Surface steps refined so the low-chrome hierarchy reads clearly.
- **`caffeine`** — warmer **coffee / cream** identity. Warmer brown/cream
  surfaces in light mode, warmer espresso surfaces in dark mode, a warmer
  caramel/copper accent, and canvas tokens shifted to the same warm family.
  Radius `0.375rem` (tighter, more "crafted" feel).

All foreground/background pairs in both modes were checked with an automated
WCAG 2.2 AA contrast calculation (see
[visual-contrast-report.md](visual-contrast-report.md)); all required pairs
pass. Whether the presets are *visibly* distinct to a user is a manual item.

### Derived picker swatches

Picker `swatches` are no longer a second hardcoded colour list. They are
derived in `manifest.ts` from each normalized fixture's **dark-mode** token
map, in this exact order: `preset.dark.primary`, `preset.dark.background`,
`preset.dark['muted-foreground']`. This matches the previous hardcoded values
(amber `#f59e0b` / `#070b14` / `#93a4bd`; caffeine `#d6a87c` / `#181512` /
`#a8a29e`) so the picker preview stays visually consistent while the duplication
is removed.

### Documented colour exceptions

The only hardcoded styling hex values that remain intentionally are the
`zone_tints` overlay defaults (native overlay independence), the documented
`ZoneCanvas.svelte` fallback, and test-fixture hex. These are not tokenized.

## Appearance preferences are per-browser-profile

Mode (`system | light | dark`) and preset selection are owned by ModeWatcher's
persistent preferences in the browser's `localStorage`, using the documented keys:

- `mode-watcher-mode`
- `mode-watcher-theme`

Key points (also stated in the Appearance settings UI):

- Preferences are stored **in this browser profile only**. They do not sync
  across devices, browsers or browser profiles, and are not part of the workspace
  config file (`ScreenLayout.json`).
- They are **not** part of the workspace draft: changing mode or preset does not
  dirty the draft, does not affect Save/Discard, and does not create undo/redo
  history. `AppearanceSettings.svelte` imports no store/API modules and never
  calls `setSettings` / `setAppOptions` / `save`.
- `system` mode follows the OS `prefers-color-scheme` setting.
- `frontend/index.html` is the single pre-paint initialization owner: it reads
  and validates both keys, resolves `system` via `matchMedia`, and sets `.dark`,
  `color-scheme` and `data-theme` on `<html>` before the app paints. `ModeWatcher`
  in `App.svelte` uses `disableHeadScriptInjection={true}` and the same keys, so
  it cannot introduce a competing init script. Invalid/missing values fall back
  to `system` / `zoneflow-amber`.

## Overlay independence

Editor theme mode/preset changes **do not** alter the native drag-to-snap overlay.
Native overlay colors remain independent workspace settings, controlled by
`zone_tints`, `overlay_inactive_alpha` and `overlay_active_alpha` in the Settings
view (stored in `ScreenLayout.json` via the existing settings flow). No
overlay-specific tokens exist in preset JSON, and no theme preference is written
to Rust config/DTOs/API.

## Accessibility notes

### Reduced motion

Non-essential transitions on the modernized primitives use Tailwind's
`motion-reduce:transition-none` so they are disabled when the OS/browser reports
`prefers-reduced-motion: reduce`. This is present on `button`, `tabs`, `switch`
(track and thumb) and `resizable`. Verifying the actual behaviour requires
enabling the OS/browser reduced-motion setting in an interactive session — see
the [manual verification checklist](manual-verification-checklist.md).

### Keyboard operation

WP3 added DOM-free keyboard/focus helpers in
`frontend/src/lib/components/ui/a11y-helpers.ts` (focus-trap Tab decisions,
roving-tabindex steps, enabled-item navigation, Home/End edges, Escape dismiss).
These are covered by unit tests in `frontend/tests/ui-a11y-helpers.test.ts`.
Modernized dialog, context menu, tabs, tooltip and switch wire these helpers to
DOM focus. A keyboard-only pass in a real browser is still required (AC8).

### AC7 / AC8: automated/static evidence vs manual verification

| Criterion | Verified by automated / static evidence | Still requires manual verification |
|---|---|---|
| **AC7** — WCAG 2.2 AA contrast (4.5:1 normal text, 3:1 large text, 3:1 meaningful non-text UI/graphical objects) across all presets/modes | Fixture/generated-CSS tests confirm token completeness. In VWP6 an **automated contrast calculation** was run over every required pair for both presets and both modes; all required text (4.5:1) and non-text/canvas-state (3:1) pairs pass. Three exploratory `input`-boundary pairs in dark mode fall just below 3:1 and are reported as findings in [visual-contrast-report.md](visual-contrast-report.md). This is a calculation, **not** visual verification. | Yes — visual review of each preset/mode (including canvas states, focus rings, scrim readability and the reported boundary findings) in an interactive session. |
| **AC8** — visible keyboard focus, keyboard operation, reduced motion | Unit tests cover the keyboard/focus math (`ui-a11y-helpers.test.ts`); `motion-reduce:transition-none` classes are present statically; `bun run check` and `bun test` pass. | Yes — keyboard-only pass (Tab/Shift+Tab, arrows, Home/End, Escape, Enter/Space), visible focus in light/dark, and reduced-motion behaviour with the OS/browser setting enabled. |

### Explicitly deferred / known limitations

These are intentionally out of scope for the first implementation and must not be
described as accessibility-complete:

- **Resizable keyboard resize** — `resizable.svelte` has no keyboard resize. The
  handle is deliberately not a tab stop (a non-functional tab stop would be worse
  than none). Pointer/touch resize is unchanged.
- **DOM-level component tests** — no DOM test harness was added. Keyboard/focus
  behaviour is covered by pure helper unit tests plus manual verification, not by
  DOM-level component tests.
- **Dialog `inert` / body scroll-lock** — `dialog.svelte` provides a scrim, focus
  trap, focus-in and focus restore, but background `inert` and body scroll-lock
  are intentionally deferred; the rest of the document is not mutated.
- **Native overlay label legibility** — Win32 overlay text/labels are visual-only
  and are not exposed through browser accessibility APIs.
- **Snap-success feedback** — not implemented; no fake success indicator is
  shown (no confirmed worker-to-native confirmation path).
- **Keyboard zone selection during drag** — deferred until separately specified.

See the [plan's Accessibility Limitations section](plans/zoneflow-overlay-ux-theme-plan.md#accessibility-limitations-explicit)
for the authoritative scope statement.

## Official shadcn-svelte component CSS additions (O1–O6)

The migration of the hand-rolled `ui/*` primitives to official shadcn-svelte /
Bits UI components required a few **additive** rules in
`frontend/src/app.css`. These are base/scaffolding additions only — **no preset
JSON value and no generated CSS (`src/themes/generated/presets.css`) was
changed**, and the existing token documentation above is unaffected:

- `@import "tw-animate-css";` — provides the `animate-in`, `fade-in-0`,
  `zoom-in-95` and `slide-in-*` utilities used by generated overlay components.
- `@custom-variant dark (&:is(.dark *));` — makes generated `dark:` utilities
  follow the app's `:root.dark` class (mode-watcher) instead of the OS
  `prefers-color-scheme`.
- A radius scale in `@theme inline` (`--radius-sm` … `--radius-4xl`) derived
  from the existing `--radius`; generated nova components reference
  `var(--radius-md)`.
- Sidebar token aliases in `@theme inline` (`--color-sidebar`,
  `--color-sidebar-foreground`, `--color-sidebar-primary`,
  `--color-sidebar-primary-foreground`, `--color-sidebar-accent`,
  `--color-sidebar-accent-foreground`, `--color-sidebar-border`,
  `--color-sidebar-ring`) mapped to existing semantic tokens, for forward
  compatibility. Sidebar is **not** adopted by this migration.
- A `@layer base` cursor rule (`button:not(:disabled)`,
  `[role="button"]:not(:disabled) { cursor: pointer; }`) to restore the pointer
  cursor under Tailwind v4.
- A global `@media (prefers-reduced-motion: reduce)` rule in `@layer base`
  covering transitions/animations inside the official components.

The generated dialog overlay uses `bg-scrim` (the generated `bg-black/10`
default was replaced), so the scrim remains the canonical token described in
[The `scrim` token](#the-scrim-token-new-required) above. Visual verification
of these additions in an interactive session is **NOT YET PERFORMED** — see the
[official-components migration section](manual-verification-checklist.md#official-components-migration-o1o6--manual-items).

## Directional shadow tokens (visual implementation V4)

The visual implementation added five **named directional shadow tokens** in
`frontend/src/app.css` to replace decorative borders with surface/shadow
separation. They are defined in `:root` and `:root.dark` (following the existing
`--zf-shadow-color` light/dark pattern) and mapped in `@theme` so the Tailwind
utilities exist:

| Token | `@theme` mapping | Utility | Role |
|---|---|---|---|
| `--zf-shadow-topbar` | `--shadow-zf-topbar` | `shadow-zf-topbar` | Top bar and tool rows |
| `--zf-shadow-rail` | `--shadow-zf-rail` | `shadow-zf-rail` | Left rail |
| `--zf-shadow-inspector` | `--shadow-zf-inspector` | `shadow-zf-inspector` | Inspector wrapper |
| `--zf-shadow-status` | `--shadow-zf-status` | `shadow-zf-status` | Status bar |
| `--zf-shadow-inset-ring` | `--shadow-zf-inset-ring` | `shadow-zf-inset-ring` | Nav/segmented groups and inspector empty state |

These are structural additions in `app.css` only. **No preset JSON value and no
generated CSS (`src/themes/generated/presets.css`) was changed**; the existing
token documentation above is unaffected. `bun run themes:check` confirms the
generated CSS is current. Visual verification of these shadows in an
interactive session is **NOT YET PERFORMED** — see the visual implementation
section of
[manual-verification-checklist.md](manual-verification-checklist.md).

## Typography (UI polish fix P6)

The UI polish fix standardises typography on a dense-interface type scale —
**12px metadata, 14px body/secondary, 16px primary and 20px section heading**,
with two weights (400/600) and body line-height 1.4–1.6. These are applied with
the existing Tailwind text steps (`text-xs` / `text-sm` / `text-base` /
`text-xl`) and the existing semantic colour tokens (`--zf-faint` / `--zf-muted`
for secondary text); off-scale font-size literals were eliminated. **No preset
JSON value and no generated CSS (`src/themes/generated/presets.css`) was
changed**, and no new colour token was introduced — the existing token
documentation above is unaffected. `bun run themes:check` confirms the generated
CSS is current. Visual verification of typography and contrast in both modes is
**NOT YET PERFORMED** — see the UI polish fix section of
[manual-verification-checklist.md](manual-verification-checklist.md).
