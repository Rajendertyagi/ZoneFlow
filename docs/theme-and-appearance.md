# Theme, presets and appearance preferences

This document describes the editor theme system delivered by WP1–WP4 of the
[ZoneFlow Overlay UX + Theme plan](plans/zoneflow-overlay-ux-theme-plan.md). It is
documentation only — it does not change application behaviour. Bun is a
**build-time tool only**; the installed app has no Node/Bun runtime.

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
  `success`, `success-foreground`, `warning`, `warning-foreground`) and the
  canvas-specific tokens (`canvas-bg`, `canvas-grid`, `canvas-zone-stroke`,
  `canvas-zone-selected`, `canvas-zone-drop`, `canvas-zone-merge`,
  `canvas-handle-shared`, `canvas-handle-outer`, `canvas-preview`,
  `canvas-label-bg`, `canvas-label-fg`). `chart-1`…`chart-5` are optional.
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
| **AC7** — WCAG 2.2 AA contrast (4.5:1 normal text, 3:1 large text, 3:1 meaningful non-text UI/graphical objects) across all presets/modes | Fixture tests confirm every required token is present and well-formed for every preset and both modes; generated-CSS tests confirm complete token blocks. **No contrast-ratio checker was run in WP6**, so contrast compliance is **not** established by automated evidence. | Yes — run a contrast checker/tooling and review each preset/mode, including canvas states and focus rings. |
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
