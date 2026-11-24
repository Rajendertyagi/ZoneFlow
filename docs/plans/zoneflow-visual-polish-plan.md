# ZoneFlow Visual Polish — Low-Chrome Design Pass + Preset Distinctness

## Status

- **Plan type**: implementation plan. This file is documentation only; it changes no application code.
- **Repository**: `Rajendertyagi/ZoneFlow` (local checkout `D:\Temp\ScreenLayoutRust`, Windows/PowerShell).
- **Baseline**: `main` at **`ba9b393`** (clean tree, in sync with `origin/main`). The thin draft's baseline `597dc60` is stale and is superseded here. WP0–WP6 of the [Overlay UX + Theme plan](zoneflow-overlay-ux-theme-plan.md) are implemented, committed, pushed and audited clean.
- **Plan directory convention**: `docs/plans/` (this file lives alongside `zoneflow-overlay-ux-theme-plan.md`, `zoneflow-feature-preservation.md`, `zoneflow-rust-axum-svelte-architecture.md`).
- **Publishing**: no PR policy file exists in the repo; recent history and the current repo workflow are direct-to-main. Publishing therefore follows the repo's existing **direct-to-main** workflow (commit + push per checkpoint). `.github/workflows/ci.yml` runs on **both `push` and `pull_request`**, so CI gates every push regardless. No PR is required by repo policy, but none is forbidden.

### Verification status (read this first)

> **No visual inspection, screenshot comparison, contrast measurement, or manual browser/desktop verification has been performed at planning time.** Every screenshot and manual item in this plan is marked **NOT YET PERFORMED**. The automated checks described here are commands to be run during implementation, not results already obtained. Nothing in this document should be read as evidence that the visual polish has been applied or validated.

## Why this plan exists

WP1–WP6 delivered the theme **infrastructure** — JSON presets, generated CSS, ModeWatcher preferences, a pre-paint bootstrap, semantic tokens, canvas tokenization, UI a11y helpers, and native overlay UX — but deliberately avoided a visual redesign. Two consequences remain:

1. **The low-chrome look was never implemented.** Panels, cards, chips, rows and toolbars still use heavy `border border-zf-border` / `border-zf-border-strong` boxed chrome, and the default preset still reads close to the old dark palette. The theme feature is therefore not visually obvious to a user.
2. **Hardcoded styling colors remain outside `Canvas/*`.** `frontend/src/lib/components/dialogs/WizardDialog.svelte` still contains `#f59e0b` (SVG fill/stroke fallback, ~lines 177/179), `#e6edf7` (SVG text, ~line 186) and a `zone_tints` fallback array (~line 34). All dialogs use a `bg-black/70` scrim. WP4 was scoped to `Canvas/*` only, so dialogs were never tokenized.

This plan closes both gaps without touching behavior, workflow, or the frozen architecture decisions.

## Intended visual direction

A **polished, modern, LOW-CHROME desktop app**: calmer surfaces, clearer hierarchy, less visual clutter, and intentional use of elevation, spacing, contrast and subtle shadows in place of stacked outlines.

**"Low-chrome" does NOT mean "borderless at all costs."** Useful borders and affordances are preserved and, where necessary, strengthened:

- input/select boundaries and focus rings (keyboard accessibility),
- drag handles and resize grips (discoverability),
- meaningful separators between distinct regions,
- status distinctions (selected / dirty / error / success / disabled),
- canvas zone strokes, handles, previews and guides (spatial meaning).

The goal is to remove **decorative** boxed chrome — the repeated `rounded-lg border border-zf-border bg-zf-surface` wrapper on every panel, card, chip and row — not to remove functional boundaries. Where a boundary is removed, it must be replaced by a deliberate surface-contrast step or a subtle elevation shadow so the grouping remains legible.

Design references `docs/design/design-mockup.html`, `docs/design/design-v2.html` and `docs/design/design-v3.html` are **historical inspiration only** — they are never loaded by the app. `design-v3.html` contributes a useful vocabulary: `--sh-1/2/3` elevation shadows, `--line: rgba(255,255,255,.045)` low-chrome hairlines, radii `--r-sm/md/lg/xl`, and a deep-slate + warm-amber palette. These are ideas to translate into the semantic token system, not values to copy verbatim.

## Locked decisions (resolved — not open questions)

These five decisions resolve every open question in the thin draft. They are firm inputs to implementation.

1. **Scrim token — ADD.** Add a `scrim` token to `frontend/src/themes/schema.ts` as a **required** key in `REQUIRED_LIGHT_DARK_KEYS` (therefore required in `light` and `dark` for **both** presets), add values to both preset JSON files, regenerate `frontend/src/themes/generated/presets.css`, add `--color-scrim: var(--scrim);` to the `@theme inline` block in `frontend/src/app.css` (this is what makes the `bg-scrim` utility resolvable), and replace `bg-black/70` with `bg-scrim` in dialogs. `bg-scrim` is the **canonical utility form**; `bg-[var(--scrim)]` arbitrary-value syntax is not used. `var(--scrim)` is reserved for contexts where a Tailwind utility cannot be applied. Justification: this is a small, documented schema extension that makes dialogs theme-aware and removes the last styling exception outside the overlay/test-data carve-outs. It is intentionally narrow (one token, two fixtures, one `@theme inline` mapping, regenerated CSS) and is covered by the existing schema/generated-CSS tests.
2. **Elevation/shadow tokens — define in `app.css` ONLY.** Define elevation as CSS custom properties in `frontend/src/app.css` around a dedicated dark shadow base that is **dark in both modes** with mode-specific strength (e.g. `:root { --zf-shadow-color: rgb(15 23 42 / 0.16); }` for light mode and `:root.dark { --zf-shadow-color: rgb(0 0 0 / 0.55); }` for dark mode, then `--zf-shadow-1: 0 1px 2px var(--zf-shadow-color);`, `--zf-shadow-2: 0 4px 14px var(--zf-shadow-color);`, `--zf-shadow-3: 0 18px 48px var(--zf-shadow-color);`), **not** in the preset JSON schema. Shadows **must be dark in both modes** and **must NOT be derived from `--foreground`**, which is near-white in dark mode and would produce light glows instead of natural shadows, undermining surface hierarchy. `--zf-hairline` may still be derived from `--foreground` via `color-mix`; a light hairline in dark mode is intentional for edge definition and is **not** a shadow. Rationale: shadows are structural, not per-preset identity; keeping them out of the schema avoids generator churn, fixture duplication and a second source of truth.
3. **Preset distinctness — meaningfully but conservatively distinct.** Change accent hue, surface temperature and canvas tokens while preserving WCAG 2.2 AA contrast. `zoneflow-amber` stays amber/deep-slate; `caffeine` becomes a clearly warmer **coffee/cream** identity (warmer surfaces, warmer accent, warmer canvas). Exact values **must** be validated with a contrast checker and visual inspection; this is explicitly marked as **requiring visual verification** and is not claimed as done here.
4. **Manifest swatches — DERIVE.** Replace the hardcoded hex arrays in `frontend/src/themes/manifest.ts` with values derived from the **dark-mode** token map of the normalized fixture, in this exact order: `preset.dark.primary`, `preset.dark.background`, `preset.dark.muted-foreground`. This matches the current hardcoded swatches (amber: `#f59e0b` / `#070b14` / `#93a4bd`; caffeine: `#d6a87c` / `#181512` / `#a8a29e`) so the picker preview stays visually consistent. This removes the last hardcoded swatch duplication. The `entry(...)` helper already calls `normalizePreset(fixture)`; derive swatches from that result's `dark` map rather than from a second literal list.
5. **Documented exceptions remain.** Keep `zone_tints` defaults (SettingsView / ZoneCanvas) and test-fixture hex (e.g. `preference.test.ts`) as documented exceptions: overlay independence and test data, respectively. Do not tokenize them.

## Frozen constraints (carry over — must not regress)

Verified from `docs/plans/zoneflow-feature-preservation.md` and `docs/plans/zoneflow-rust-axum-svelte-architecture.md`:

- **D1** — Apply (arrange running windows) and Launch & Arrange (explicit launch) remain distinct actions.
- **D2** — Dirty guard, `409 dirty_state`, no silent discard.
- **D3** — Replace assignment preserves geometry/ID and resets rules only with confirmation.
- **D4** — Disable suspends all automation and is runtime-only.
- **D5** — Real execution plus a separate Preview; no simulated success.
- **D6** — Rail = load draft; Home = select only; tray = apply when enabled / select-only when disabled.
- Appearance preferences stay in ModeWatcher `localStorage` (`mode-watcher-mode`, `mode-watcher-theme`), never in workspace `Settings`/`SettingsDto`; no dirty/undo impact.
- Generated CSS from JSON is the source of truth; the pre-paint bootstrap in `frontend/index.html` contains IDs only, never token values.
- Native overlay colors are independent of editor theme (`zone_tints` / overlay alpha via settings). Preserve the `MoveWindow` authority boundary: native sends `zone_id`; the worker re-derives the rect.
- No new framework/runtime, no backend API changes, no unnecessary dependencies, no user theme import.
- **WCAG 2.2 AA**: 4.5:1 normal text, 3:1 large text, 3:1 meaningful non-text UI/graphical objects; visible focus; reduced motion.

## What changes and what remains unchanged

| Area | Changes | Remains unchanged |
|---|---|---|
| Theme schema | Add required `scrim` token; keep literal allowed-key types | All existing required/optional keys, normalization/validation rules, tweakcn-compatible fixture shape |
| Preset JSON | Adjust accent/surface/canvas values for distinctness; add `scrim` to both modes | Required token completeness, kebab-case `name`, `registry:style` type, radius/font shared keys |
| Generated CSS | Regenerate from fixtures | Generated file stays committed and staleness-checked; bootstrap stays IDs-only |
| Manifest | Derive swatches from normalized tokens | `id`, `label`, default/fallback lookup helpers |
| `app.css` | Add `--color-scrim: var(--scrim);` to `@theme inline` (VWP1); add elevation/shadow custom properties and low-chrome base rules (VWP2) | Other `@theme inline` semantic mappings, `zf-*` compatibility aliases, body base styles |
| Dialogs | Replace hardcoded hex with tokens; `bg-black/70` → `bg-scrim` | Focus trap, Escape dismiss, focus restore, ARIA roles, deferred `inert`/scroll-lock |
| Shell & views | Reduce decorative borders; use surface/elevation hierarchy | All behavior, state, D1–D6 workflow, keyboard semantics, `motion-reduce:transition-none` |
| Canvas | Refine opacity/handles/labels/guides/empty states using existing tokens | Tokenized colors from WP4, geometry, snapping, `zone_tints` fallback at `ZoneCanvas.svelte:82` |
| Docs | Update theme doc + manual checklist; add this plan | Historical design references remain inspiration-only |
| Backend / Rust | **Nothing** | All Rust code, DTOs, config, API, native overlay logic |

## Relevant screens and components inventory

Current state is from source inspection at baseline; intended treatment is the target of this plan.

| Screen / component | File | Current state | Intended treatment |
|---|---|---|---|
| App shell | `frontend/src/App.svelte` | Root flex column `h-screen`; TopBar; view container; global dialogs; toasts fixed bottom-right with border + `shadow-xl`; connection-lost banner bordered | Keep layout; soften toast/banner chrome via elevation tokens; keep status legibility |
| Top bar | `frontend/src/lib/components/TopBar.svelte` | `border-b`; brand; nav in bordered rounded container with 4 buttons; **four** bordered status chips (applied/selected/saved/layout) each `rounded border border-zf-border bg-zf-surface px-2 py-1`; connection Badge; close button | Low-chrome nav container (surface step + subtle shadow); status chips become tinted/subtle pills with semantic status color, not four identical boxes; preserve close button and Badge |
| Home view | `frontend/src/lib/views/HomeView.svelte` | Status hero `rounded-lg border border-zf-border bg-zf-surface` with left accent bar; action bar with `border-t`; profile list `overflow-hidden rounded-lg border border-zf-border` with bordered rows and many buttons | Hero becomes elevated surface with accent bar retained; profile rows lose per-row borders in favor of hover/selected surface steps; keep action bar separator |
| Editor view | `frontend/src/lib/views/EditorView.svelte` | Canvas-tools toolbar `border-b border-zf-border bg-zf-surface`; action bar `border-b border-zf-border bg-zf-bg`; ApplyResults; three-pane workspace: left rail `w-60 border-r border-zf-border bg-zf-surface` (ProfilesPanel top 2/5, AppLibrary bottom), main canvas `rounded-lg border border-zf-border bg-zf-surface shadow-[0_0_0_1px_rgba(0,0,0,0.2)]`, inspector `w-80` (ZoneInspector has `border-l`); StatusBar `border-t`; many `h-5 w-px bg-zf-border` separators | Replace boxed panes with surface-contrast + elevation; keep meaningful separators and the canvas frame (as a subtle shadow, not a double border); keep pane widths and resizing |
| Profiles panel | `frontend/src/lib/components/LeftRail/ProfilesPanel.svelte` | Bordered profile cards; selected `border-zf-accent/60 bg-zf-accent/10`; hover `border-zf-border-strong` | Selected state becomes accent-tinted surface + accent hairline; hover becomes surface step; keep selection unmistakable |
| App library | `frontend/src/lib/components/LeftRail/AppLibrary.svelte` | Tabs (Installed/Running), search Input, app list `rounded-md border border-zf-border bg-zf-surface` with bordered rows | List container loses outer box; rows use hover/selected surface steps; keep Tabs and search Input affordances |
| Zone inspector | `frontend/src/lib/components/Inspector/ZoneInspector.svelte` | `aside border-l`; header `border-b`; Tabs (Process/Browser/Macro/Console); read-only fields as bordered boxes; launcher box bordered; log box bordered | Keep `border-l` pane divider and header separator; read-only fields become subtle inset surfaces; launcher/log boxes use elevation; keep Tabs |
| Settings view | `frontend/src/lib/views/SettingsView.svelte` | 4 bordered sections (Editor, Startup & close, Overlay, Advanced) each `rounded-lg border border-zf-border bg-zf-surface p-4`; many bordered inputs; `AppearanceSettings` rendered **outside** the loaded-draft block | Sections become elevated cards with hairline separators; keep input borders; keep `AppearanceSettings` placement and no-dirty behavior |
| Hotkeys view | `frontend/src/lib/views/HotkeysView.svelte` | Bordered list of bindings with Inputs | Rows lose per-row boxes; keep Inputs and `kbd` styling; use separators |
| Apply results | `frontend/src/lib/components/ApplyResults.svelte` | Bordered top strip; assignment chips bordered | Strip becomes surface step; chips become subtle tinted pills; keep status colors |
| Status bar | `frontend/src/lib/components/StatusBar.svelte` | Bordered bottom strip | Keep `border-t` as a meaningful region separator; reduce inner chrome |
| Dialogs | `frontend/src/lib/components/dialogs/{WizardDialog,PickerDialog,ConfirmDialog,PromptDialog,CloseDialog}.svelte`, `frontend/src/lib/components/ui/dialog.svelte` | `bg-black/70` scrim + `border border-zf-border-strong bg-zf-panel shadow-2xl`; WizardDialog hardcoded `#f59e0b`, `#e6edf7`, tints fallback | Scrim → `bg-scrim`; panel uses elevation tokens; WizardDialog hex → semantic tokens; keep tints fallback exception; keep focus trap/ARIA |
| Canvas | `frontend/src/lib/components/Canvas/{ZoneCanvas,ZoneRect,SnapGuides,DragPreview}.svelte` | Tokenized in WP4; only documented `zone_tints` fallback hex at `ZoneCanvas.svelte:82` | Refine opacity, handle size/contrast, label legibility, guide subtlety, empty-state treatment; no new hardcoded colors |
| UI primitives | `frontend/src/lib/components/ui/*` (button, input, select, badge, tabs, switch, tooltip, context-menu, resizable, separator, scroll-area, label, slider) | WP3 a11y + `motion-reduce:transition-none` on button/tabs/switch/resizable | Selective low-chrome styling only; preserve focus rings, disabled states, keyboard helpers, reduced motion |
| Appearance settings | `frontend/src/lib/components/AppearanceSettings.svelte` | Mode/preset controls in Settings | No behavior change; may gain a short note about preset identities |

## Visual design specification

### Token reference (exact names)

The following tokens are the vocabulary of this plan. All except the `app.css`-only elevation tokens live in the preset JSON fixtures and are emitted into `frontend/src/themes/generated/presets.css`.

| Token | Role | Used by |
|---|---|---|
| `--background` / `--foreground` | App background and primary text | body, shell, all views |
| `--card` / `--card-foreground` | Panel surface (rails, inspector, cards) | EditorView, panels, cards |
| `--popover` / `--popover-foreground` | Overlay surface (dialogs, menus, tooltips) | dialogs, context-menu, tooltip |
| `--secondary` / `--secondary-foreground` | Raised/interactive surface (hover, active tabs, buttons) | ui primitives, rows, chips |
| `--muted` / `--muted-foreground` / `--muted-foreground-faint` | Secondary/tertiary text and quiet surfaces | labels, metadata, read-only fields |
| `--primary` / `--primary-hover` / `--primary-foreground` | Accent action color | buttons, selected states, focus |
| `--accent` / `--accent-foreground` | Accent tint surface | selected/hover tints |
| `--border` / `--input` / `--ring` | Functional boundaries and focus | inputs, focus rings, separators |
| `--destructive` / `--success` / `--warning` (+ `-foreground`) | Status colors | status chips, toasts, results |
| `--canvas-bg` / `--canvas-grid` | Canvas background and grid | ZoneCanvas |
| `--canvas-zone-stroke` / `--canvas-zone-selected` / `--canvas-zone-drop` / `--canvas-zone-merge` | Zone states | ZoneCanvas, ZoneRect |
| `--canvas-handle-shared` / `--canvas-handle-outer` | Drag handles | ZoneRect |
| `--canvas-preview` / `--canvas-label-bg` / `--canvas-label-fg` | Preview and labels | DragPreview, ZoneCanvas |
| `--chart-1` … `--chart-5` | Optional chart series | reserved |
| `--scrim` (**new, required**) + `--color-scrim: var(--scrim);` in `@theme inline` | Dialog scrim, theme-aware; the `@theme inline` mapping is what makes the `bg-scrim` utility resolvable | `app.css` (mapping), `ui/dialog.svelte` and all dialogs (consumers) |
| `--zf-shadow-color`, `--zf-shadow-1/2/3`, `--zf-hairline` (**new, `app.css` only**) | Dark shadow base (dark in both modes) + elevation and low-chrome lines | panels, cards, dialogs, separators |

`zf-*` compatibility aliases in `app.css` map one-to-one onto semantic destinations and remain until migration is verified complete. `bg-scrim` is the **canonical utility form** for the scrim, enabled by the `--color-scrim: var(--scrim);` mapping in `@theme inline`; do **not** use `bg-[var(--scrim)]` arbitrary-value syntax and do not introduce a `zf-*` alias for `scrim`. `var(--scrim)` is acceptable only where a Tailwind utility cannot be applied (e.g. inline SVG/CSS).

### Surface hierarchy

Define a four-level hierarchy using existing semantic surfaces plus elevation tokens from `app.css`:

1. **App background** — `--background` (`bg-background` / `bg-zf-bg`). The deepest canvas.
2. **Panel surface** — `--card` (`bg-card` / `bg-zf-surface`). Rails, inspector, toolbars, cards. Distinguish from background by a subtle luminance step and `--zf-shadow-1`.
3. **Raised / interactive surface** — `--secondary` (`bg-secondary` / `bg-zf-raised`). Hover rows, active tabs, buttons, chips.
4. **Overlay surface** — `--popover` (`bg-popover` / `bg-zf-panel`). Dialogs, menus, tooltips; strongest elevation (`--zf-shadow-3`).

Rule: adjacent levels must be distinguishable without a border. If two levels are too close in a given preset/mode, adjust the preset surface tokens (VWP5) rather than reintroducing outlines.

### Borders and lines

- Remove decorative `border border-zf-border` wrappers from panels, cards, list containers, chips and rows.
- Replace with surface contrast + `--zf-shadow-1/2` where grouping is needed.
- **Keep** `border` / `border-zf-border-strong` on: inputs, selects, text areas, focus rings, `kbd`, drag/resize handles, and the canvas frame (as a single subtle shadow ring, not a double border).
- **Keep** meaningful separators: TopBar `border-b`, action-bar `border-t`, inspector `border-l`, StatusBar `border-t`, section header dividers. Prefer a low-chrome hairline (design-v3 `--line` idea) over a full-strength border where the separator is purely visual.
- Never remove a boundary that communicates state (selected, dirty, error, disabled).

### Spacing

- Standardize on a 4px-based rhythm: 4 / 8 / 12 / 16 / 24px (`p-1`…`p-6` equivalents).
- Give removed borders back their breathing room: when a boxed row loses its border, add ~2px vertical padding so hit targets do not shrink.
- Maintain a minimum interactive target of **24 × 24 CSS pixels (both width and height)** for dense desktop controls; do not shrink targets to achieve a cleaner look. Exceptions: purely visual separators (e.g. `h-5 w-px bg-zf-border` dividers) are not interactive and are exempt; inline text links inside prose are exempt. Existing controls that are already larger (e.g. `h-7` = 28px, `h-9 w-9` = 36px) already satisfy the floor. When a bordered row loses its border, preserve its padding so the target does not shrink below 24 × 24.
- Keep pane widths (`w-60` rail, `w-80` inspector) and the 2/5 ProfilesPanel split unchanged.

### Typography

- Keep the existing font stacks (`font-sans`, `font-mono` from preset shared keys) and base `font-size: 0.875rem`.
- Use weight, letter-spacing and muted-foreground levels for hierarchy instead of boxes: section labels uppercase/tracked (design-v3 `.cap` idea), values in `foreground`, secondary text in `muted-foreground`, tertiary in `muted-foreground-faint`.
- Keep tabular numerals for coordinates/status where already used.

### Colors

- All styling colors come from semantic tokens in `generated/presets.css` via `@theme inline` mappings in `app.css`; `zf-*` aliases remain for compatibility.
- Status colors (`success`, `warning`, `destructive`, `primary`) keep their existing semantic destinations and contrast pairings.
- Selected/hover states use `color-mix(in oklab, var(--primary) … , transparent)` or existing accent tokens — never a new hardcoded hex.
- The only allowed hardcoded hex after this plan: `zone_tints` defaults, the documented `ZoneCanvas.svelte:82` fallback, and test fixtures.

### Depth and elevation

Define in `frontend/src/app.css` (not in preset JSON) a dedicated dark shadow base that is **dark in both modes** with mode-specific strength, then build the three elevation levels from it:

- `--zf-shadow-color` — dark shadow base: `:root { --zf-shadow-color: rgb(15 23 42 / 0.16); }` (light mode: subtle dark shadow) and `:root.dark { --zf-shadow-color: rgb(0 0 0 / 0.55); }` (dark mode: stronger dark shadow, still dark — not a glow).
- `--zf-shadow-1: 0 1px 2px var(--zf-shadow-color);` — subtle resting elevation for panels/cards (design-v3 `--sh-1` idea).
- `--zf-shadow-2: 0 4px 14px var(--zf-shadow-color);` — hover/raised elevation (design-v3 `--sh-2` idea).
- `--zf-shadow-3: 0 18px 48px var(--zf-shadow-color);` — dialogs/menus/tooltips (design-v3 `--sh-3` idea).
- Optionally `--zf-hairline` — a low-chrome line derived from `--foreground` via `color-mix`, for purely visual separators. A light hairline in dark mode is intentional for edge definition and is **not** a shadow.

Shadows **must be dark in both modes** and **must NOT be derived from `--foreground`** (near-white in dark mode, which would yield light glows instead of natural shadows and undermine surface hierarchy). Map them to Tailwind utilities via `@theme` if utility classes are preferred, but keep their definitions in `app.css` only — not in preset JSON.

### Controls

- Buttons: keep `motion-reduce:transition-none`; use raised surface + `--zf-shadow-1` at rest, `--zf-shadow-2` on hover; primary buttons keep `--primary`/`--primary-foreground`.
- Inputs/selects: **keep visible borders** (`border-input`) and focus rings (`ring`); this is an accessibility affordance, not decorative chrome.
- Tabs/switch/tooltip/context-menu: preserve WP3 keyboard helpers and focus semantics; only restyle surfaces.
- Chips/badges: replace bordered boxes with tinted surfaces (`color-mix` of the relevant semantic color) while keeping status distinctions legible.
- Disabled states: keep reduced opacity/cursor and ensure they remain distinguishable (do not rely on color alone).

### Navigation

- TopBar nav container: surface step + subtle shadow instead of a bordered rounded box; keep the 4 buttons and their active state clearly indicated.
- View switching (Home / Editor / Settings / Hotkeys) keeps its current mechanism and active indicator; only the chrome changes.

### Dialogs

- Scrim: use the canonical `bg-scrim` utility (enabled by `--color-scrim: var(--scrim);` in `@theme inline`; `var(--scrim)` only where a utility cannot be applied) from the new required token; must remain dark enough for readable dialog text in **light** presets and not crush contrast in dark presets — validate visually and with contrast checks.
- Panel: `bg-popover` + `--zf-shadow-3`; keep `rounded-lg`, `p-6`, ARIA roles, focus trap, focus-in/restore, Escape dismiss.
- WizardDialog: `#e6edf7` → `var(--foreground)` (or `var(--canvas-label-fg)` where canvas-like); `#f59e0b` fallback → `var(--primary)` / `var(--canvas-preview)`; keep the `zone_tints` fallback array as a documented overlay exception.
- Keep the documented limitation that background `inert` and body scroll-lock are deferred.

### Canvas treatment

- Colors are already tokenized (WP4); do not introduce new hardcoded hex.
- Refine: zone fill opacity, stroke weight, handle size/contrast (`canvas-handle-shared`, `canvas-handle-outer`), label background/foreground legibility (`canvas-label-bg`, `canvas-label-fg`), snap-guide subtlety (`SnapGuides.svelte`), drag-preview clarity (`DragPreview.svelte`), and the empty-state treatment for a blank canvas.
- Selected / drop / merge / handle states must remain mutually distinguishable in every preset and mode (AC9).
- Keep the `zone_tints` fallback at `ZoneCanvas.svelte:82` as the sole documented canvas hex exception.

## How presets become meaningfully distinguishable

Both presets currently have complete token sets but differ subtly in hue and radius, so switching is not obvious. VWP5 will:

- **zoneflow-amber** — keep the amber/deep-slate identity: dark background near `#070b14`, amber primary near `#f59e0b`, radius `0.5rem`. Refine surface steps so the low-chrome hierarchy reads clearly.
- **caffeine** — become a clearly warmer **coffee/cream** identity: warmer brown/cream surfaces in light mode, warmer espresso surfaces in dark mode, a warmer caramel/copper accent, and canvas tokens shifted to the same warm family. Radius stays `0.375rem` (tighter, more "crafted" feel).
- Change **accent hue, surface temperature and canvas tokens** together so the difference is visible in surfaces, accent and canvas — not just a button color.
- Preserve AA contrast for every foreground/background pair in both modes; prefer adjusting surface/background hues over weakening text contrast.
- Add `scrim` values appropriate to each preset/mode.
- **Exact values are not fixed in this plan.** They must be chosen with a contrast checker and confirmed by visual inspection; this is explicitly **requiring visual verification** (NOT YET PERFORMED).
- Derive manifest swatches from the **dark-mode** token map of the normalized fixture, in this exact order: `preset.dark.primary`, `preset.dark.background`, `preset.dark.muted-foreground`, so the picker preview always matches the actual preset and stays consistent with the current hardcoded swatches.

## Accessibility and usability requirements

- **Contrast**: WCAG 2.2 AA — 4.5:1 normal text, 3:1 large text, 3:1 meaningful non-text UI components and graphical objects (including canvas states, focus rings, status chips, handles). Run an actual contrast checker over every required pair for every preset/mode; record results. This closes the AC7 gap explicitly noted as "not established" in WP6.
- **Focus**: visible `:focus-visible` rings on all interactive elements in light and dark; never remove a focus ring when removing a border.
- **Keyboard**: preserve WP3 helpers (`a11y-helpers.ts`) and keyboard semantics in dialog, context-menu, tabs, tooltip, switch; Tab/Shift+Tab, arrows, Home/End, Escape, Enter/Space must keep working.
- **Reduced motion**: any new transition must carry `motion-reduce:transition-none`; verify with the OS/browser setting enabled.
- **Disabled states**: distinguishable by more than color; keep cursor/opacity conventions.
- **Hit targets**: minimum interactive target of **24 × 24 CSS pixels (both width and height)** for dense desktop controls; do not shrink targets when removing borders. Exempt: purely visual separators (e.g. `h-5 w-px bg-zf-border` dividers) and inline text links inside prose. Existing larger controls (`h-7` = 28px, `h-9 w-9` = 36px) already satisfy the floor; when a bordered row loses its border, preserve padding so the target stays at least 24 × 24.
- **Status distinctions**: selected/dirty/error/success/disabled must not rely on border presence alone after de-boxing.
- **Zoom**: check at 200% zoom that low-chrome surfaces remain distinguishable.

## Work packages (VWP0–VWP6)

One WP at a time, in order. Each has a reviewable checkpoint. Dependencies are strict unless noted.

### VWP0 — Visual baseline & spec lock

- **Scope**: capture baseline screenshots per the documented matrix; finalize token decisions; no application code changes.
- **Files**: new screenshot artifacts under a documented location (e.g. `docs/screenshots/visual-polish/`); this plan file may be refined with confirmed values. **No app code.**
- **Behavior**: capture the current UI at baseline `ba9b393` across the core matrix subset listed in `fixture.md` (see Screenshot comparison procedure); remaining matrix cells are deferred to manual verification with rationale. Confirm the five locked decisions and the surface hierarchy. Record exact preset values only after contrast validation.
- **Fixture setup (deterministic — existing app capabilities only, no new features)**: before any capture, fix the browser viewport (e.g. maximize to a recorded size such as 1440×900, zoom 100%) and record it. Create a profile named `Visual QA` via the existing New profile prompt. Apply the `Duo` template via the existing Layout wizard (deterministic 2 zones) — use this one fixed template choice everywhere. Assign fixed apps to zones via the existing picker/Browse flow; record the exact app labels and paths in a committed fixture manifest (e.g. `docs/screenshots/visual-polish/fixture.md`) so before/after use identical data; if a chosen app is unavailable on the machine, record the substitute used and keep it identical across captures. Selected state: click zone 1 to select it (deterministic). Drop state: drag the recorded app from the library over zone 2 and capture while hovering BEFORE release; document the exact interaction. Merge state: use the existing Merge toolbar action, click zone 1 as source then hover zone 2 as target; document the exact interaction. Set mode/preset via the existing Appearance controls before each capture; capture both presets × both modes. Commit the fixture manifest alongside screenshots; if any state cannot be reproduced identically, document the deviation rather than claiming a valid comparison. **NOT YET PERFORMED.**
- **Tests**: none (documentation/screenshot only).
- **Manual verification**: **NOT YET PERFORMED** — requires an interactive Windows/browser session and the documented capture procedure.
- **Completion criteria**: baseline captures exist for the core matrix subset listed in `fixture.md`; remaining matrix cells are explicitly marked N/A or deferred to manual verification with rationale; token decisions recorded; no app code modified.
- **Regression risks**: none (no code changes).
- **Dependencies**: none. Blocks VWP1–VWP5 (before/after comparison needs a baseline).

### VWP1 — Dialog & scrim tokenization

- **Scope**: remove hardcoded styling colors from dialogs and add the `scrim` token.
- **Files**: `frontend/src/themes/schema.ts` (add `scrim` to `REQUIRED_LIGHT_DARK_KEYS`), `frontend/src/themes/presets/zoneflow-amber.json`, `frontend/src/themes/presets/caffeine.json` (add `scrim` to both `light` and `dark`), regenerate `frontend/src/themes/generated/presets.css`, `frontend/src/app.css` (add `--color-scrim: var(--scrim);` to the `@theme inline` block so `bg-scrim` resolves), `frontend/src/lib/components/dialogs/WizardDialog.svelte`, `frontend/src/lib/components/dialogs/{PickerDialog,ConfirmDialog,PromptDialog,CloseDialog}.svelte`, `frontend/src/lib/components/ui/dialog.svelte`, new static test `frontend/tests/dialog-tokens.test.ts`, extend `frontend/src/themes/generated-css.test.ts` to require `scrim` in every block.
- **Behavior**: `#e6edf7` → `var(--foreground)` (or `var(--canvas-label-fg)` where canvas-like); `#f59e0b` fallback → `var(--primary)` / `var(--canvas-preview)`; add `--color-scrim: var(--scrim);` to the `@theme inline` block in `frontend/src/app.css` and replace every `bg-black/70` scrim with `bg-scrim` (the canonical utility form — do **not** use `bg-[var(--scrim)]`; `var(--scrim)` only where a utility cannot be applied); **remove or rewrite the stale scrim-exception comment in `frontend/src/lib/components/ui/dialog.svelte`** (currently: "Scrim: `bg-black/70` is an intentional documented exception — the preset schema has no scrim token…") so it describes the new `scrim` token and the remaining documented exceptions (overlay `zone_tints` defaults and test-fixture hex); keep the `zone_tints` fallback array and test-fixture hex as documented exceptions.
- **Tests**: static test asserting no hardcoded hex in `components/dialogs/*` except documented tints fallbacks; generated-CSS test requires `scrim` in every preset/mode block; static assertion that `app.css` `@theme inline` contains `--color-scrim: var(--scrim);` and that no `bg-black/70` or `bg-[var(--scrim)]` remains in dialogs; static assertion that the stale `bg-black/70` exception comment is gone from `ui/dialog.svelte`; `bun run themes:check` passes.
- **Manual verification**: dialog scrim readability in light/dark for both presets — **NOT YET PERFORMED**.
- **Completion criteria**: no unexplained hardcoded styling colors in dialogs; `--color-scrim` mapping present in `@theme inline` and `bg-scrim` used canonically; stale scrim-exception comment removed/rewritten; all automated checks pass; no behavior/API change.
- **Regression risks**: scrim too light in light presets (contrast) or too opaque in dark presets; focus trap/ARIA regressions. Mitigate with contrast check + keyboard pass.
- **Dependencies**: VWP0 (baseline). Feeds VWP2–VWP5.

### VWP2 — Shell & navigation

- **Scope**: restyle the app shell, top bar, status bar, toasts and connection banner to low-chrome.
- **Files**: `frontend/src/app.css` (**additional to VWP1's `--color-scrim` mapping — VWP2 only adds `--zf-shadow-color`, `--zf-shadow-1/2/3`, optional `--zf-hairline` and base rules; do not remove or alter the scrim mapping**), `frontend/src/App.svelte`, `frontend/src/lib/components/TopBar.svelte`, `frontend/src/lib/components/StatusBar.svelte`, selected `frontend/src/lib/components/ui/*` (button, badge, input, separator).
- **Behavior**: replace decorative borders with surface steps + elevation; status chips become tinted pills with semantic status color; keep nav active state, close button, Badge, focus rings, disabled states, `motion-reduce:transition-none`; keep all separators that carry meaning.
- **Tests**: static/a11y checks (`bun run check`, `bun test`, existing `ux-consistency.test.ts`); no DOM harness.
- **Manual verification**: shell/nav appearance and keyboard focus in light/dark — **NOT YET PERFORMED**.
- **Completion criteria**: shell visibly lower-chrome; inputs/focus still affordant; no workflow regression.
- **Regression risks**: status chips losing distinctness; focus rings removed with borders. Mitigate by explicit status-color and focus-ring checks.
- **Dependencies**: VWP1 (shared tokens). Feeds VWP3.

### VWP3 — Views & panels

- **Scope**: apply the low-chrome treatment to all views and panels.
- **Files**: `frontend/src/lib/views/{HomeView,EditorView,SettingsView,HotkeysView}.svelte`, `frontend/src/lib/components/LeftRail/{ProfilesPanel,AppLibrary}.svelte`, `frontend/src/lib/components/Inspector/ZoneInspector.svelte`, `frontend/src/lib/components/ApplyResults.svelte`, `frontend/src/lib/components/AppearanceSettings.svelte` (documentation note only), selected `frontend/src/lib/components/ui/*` (tabs, input, select, switch, tooltip, context-menu, resizable, scroll-area).
- **Behavior**: de-box panels/cards/rows; keep pane widths and splits; keep inspector `border-l` and header `border-b`; keep input borders; keep `AppearanceSettings` outside the loaded-draft block and no-dirty behavior; preserve WP3 keyboard semantics and reduced motion.
- **Tests**: `bun run check`, `bun test`, static checks; no DOM harness.
- **Manual verification**: every view in both presets/modes; selected/hover states; 200% zoom — **NOT YET PERFORMED**.
- **Completion criteria**: views visibly lower-chrome; selection/hover/status still clear; no dirty/undo regression from Appearance.
- **Regression risks**: selected profile/app rows becoming ambiguous; settings sections losing grouping. Mitigate with surface-step contrast checks.
- **Dependencies**: VWP2. Feeds VWP4.

### VWP4 — Canvas visual refinement

- **Scope**: refine already-tokenized canvas visuals; no new hardcoded colors.
- **Files**: `frontend/src/lib/components/Canvas/{ZoneCanvas,ZoneRect,SnapGuides,DragPreview}.svelte`; optionally extend `frontend/tests/canvas-tokens.test.ts`.
- **Behavior**: tune zone fill opacity, stroke weight, handle size/contrast, label legibility, guide subtlety, drag-preview clarity, empty-state treatment using existing `canvas-*` tokens; keep the `zone_tints` fallback at `ZoneCanvas.svelte:82` as the sole documented hex exception; keep geometry/snapping and the `MoveWindow` authority boundary untouched.
- **Tests**: `bun test` (canvas token tests), `bun run check`.
- **Manual verification**: canvas states readable and distinguishable in every preset/mode (AC9) — **NOT YET PERFORMED**.
- **Completion criteria**: canvas refined with no new hardcoded colors; all states distinguishable; no snapping/geometry regression.
- **Regression risks**: handles/labels becoming too faint; preview confusion with real zones. Mitigate with contrast check and visual inspection.
- **Dependencies**: VWP3 (surface hierarchy stable). Feeds VWP5.

### VWP5 — Preset distinctness + derived swatches

- **Scope**: make presets meaningfully distinct and remove swatch duplication.
- **Files**: `frontend/src/themes/presets/zoneflow-amber.json`, `frontend/src/themes/presets/caffeine.json`, regenerate `frontend/src/themes/generated/presets.css`, `frontend/src/themes/manifest.ts`, `frontend/src/themes/schema.test.ts`, `frontend/src/themes/generated-css.test.ts`, optionally `frontend/src/lib/components/AppearanceSettings.svelte` (label copy only).
- **Behavior**: apply locked decision 3 (amber/deep-slate vs. warmer coffee/cream), add `scrim` values, derive swatches from the **dark-mode** token map of the normalized fixture in this exact order — `preset.dark.primary`, `preset.dark.background`, `preset.dark.muted-foreground` — per locked decision 4 (matching the current hardcoded swatches); keep AA contrast; keep radius/font shared keys and all required tokens.
- **Tests**: fixture schema tests, generated-CSS staleness/validity tests, contrast checker (VWP6), `bun run themes:check`.
- **Manual verification**: preset switching is immediately visible in surfaces/accent/canvas; swatches match presets — **NOT YET PERFORMED**.
- **Completion criteria**: presets visibly distinct; swatches derived; all contrast checks pass; generated CSS current.
- **Regression risks**: contrast failures; swatch derivation changing picker layout. Mitigate with contrast checker and visual inspection.
- **Dependencies**: VWP1–VWP4 (token set and surfaces stable). Feeds VWP6.

### VWP6 — Verification, contrast checker, screenshot comparison, docs

- **Scope**: full automated verification, contrast measurement, screenshot comparison, manual checklist and docs updates.
- **Files**: `docs/theme-and-appearance.md` (new `scrim` token, elevation tokens, visual direction, preset identities), `docs/manual-verification-checklist.md` (new visual items, still NOT YET PERFORMED), this plan (status updates only if needed), screenshot artifacts.
- **Automated commands — core CI-equivalent gates** (run from repo root unless noted):
  - `cd frontend && bun install --frozen-lockfile`
  - `cd frontend && bun run check`
  - `cd frontend && bun test`
  - `cd frontend && bun run build`
  - mirror `frontend/dist` → root `dist` (as CI does)
  - `cargo fmt --check`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo test`
  - `cargo build --release`
  - `cd frontend && bun run themes:check`
  - These are the **core CI-equivalent gates**. The advisory `cargo audit` step (`continue-on-error: true`) and the release artifact upload step in `.github/workflows/ci.yml` are **intentionally excluded** from this local verification list; they remain part of CI and are **not claimed to have been run here**.
- **Contrast**: run an actual WCAG 2.2 AA contrast check over every required foreground/background pair for every preset and both modes (4.5:1 normal text, 3:1 large text, 3:1 non-text UI/graphical objects). Record results per preset/mode. This closes the AC7 gap explicitly noted as "not established" in WP6.
- **Screenshot comparison**: capture the after-matrix and compare against VWP0 baseline (see procedure below).
- **Manual checklist**: add visual items (low-chrome appearance, preset distinctness, dialog scrim readability, canvas states, focus, reduced motion, 200% zoom) — all marked **NOT YET PERFORMED** unless an interactive session is actually used.
- **Completion criteria**: all automated checks pass; contrast results recorded; screenshots compared; docs updated; no false manual claims.
- **Regression risks**: false claims of manual verification; stale generated CSS. Mitigate by strict status labels and `themes:check`.
- **Dependencies**: VWP0–VWP5.

## Testing, screenshot comparison and manual visual verification

### Automated verification

Run the **core CI-equivalent gates** listed in VWP6 (the advisory `cargo audit` and release artifact upload steps are intentionally excluded from the local list but remain part of CI). CI runs on `push` and `pull_request`; the same core commands must pass locally before each checkpoint. `bun run build` regenerates CSS automatically; `bun run themes:check` fails if committed CSS is stale.

### Contrast checker

Use a real contrast-checking tool/script over the normalized token pairs for each preset and mode. At minimum check: `foreground`/`background`, `card-foreground`/`card`, `popover-foreground`/`popover`, `primary-foreground`/`primary`, `secondary-foreground`/`secondary`, `muted-foreground`/`muted` and `background`, `muted-foreground-faint` against its backgrounds, `destructive/success/warning` foreground pairs, `ring` against adjacent surfaces, and all `canvas-*` state pairs. Record pass/fail with measured ratios per preset/mode. **NOT YET PERFORMED.**

### Screenshot comparison procedure

- Root screenshots `inspect-01-default.png` … `inspect-07-bottom-crop.png` are **historical only** — they are not a valid baseline for this plan and must not be used as before-images.
- VWP0 must capture **fresh** before-images via a documented procedure: build the app, launch it on Windows, and capture the same views/states that will be captured after the work.
- **Fixture setup (deterministic — use the identical procedure for before and after captures; existing app capabilities only, no new features)**:
  1. Fix the browser viewport (e.g. maximize to a recorded size such as 1440×900, zoom 100%) and record it in the fixture manifest.
  2. Create a profile named `Visual QA` via the existing New profile prompt.
  3. Apply the `Duo` template via the existing Layout wizard (deterministic 2 zones); use this one fixed template choice everywhere.
  4. Assign fixed apps to zones via the existing picker/Browse flow; record the exact app labels and paths in a committed fixture manifest (e.g. `docs/screenshots/visual-polish/fixture.md`) so before/after use identical data. If a chosen app is unavailable on the machine, record the substitute used and keep it identical across captures.
  5. Selected state: click zone 1 to select it (deterministic).
  6. Drop state: drag the recorded app from the library over zone 2 and capture while hovering BEFORE release; document the exact interaction.
  7. Merge state: use the existing Merge toolbar action, click zone 1 as source then hover zone 2 as target; document the exact interaction.
  8. Set mode/preset via the existing Appearance controls before each capture; capture both presets × both modes.
  9. Commit the fixture manifest alongside screenshots; if any state cannot be reproduced identically, document the deviation rather than claiming a valid comparison.
- Capture matrix (each cell in both presets × both modes where applicable). The
  **VWP0 baseline** covers only the core subset listed in
  `docs/screenshots/visual-polish/fixture.md`; every other cell is explicitly
  marked **Deferred to manual verification** with a rationale. Deferred cells were
  **not** captured in VWP0 and must not be reported as captured.

  | Matrix cell | VWP0 baseline status | Notes / rationale |
  |---|---|---|
  | 1. Home view (default and maximized) | **Partially captured** — default Home in dark `zoneflow-amber` (`03`) and dark `caffeine` (`13`) | Maximized Home and light-mode Home are **deferred to manual verification** (non-core preset/mode combinations; lower priority, can be captured during VWP6 manual QA). |
  | 2. Editor view — three-pane workspace, empty canvas | **Captured** in dark `zoneflow-amber` (`01`, initial 2 unassigned zones) | Other preset/mode combinations are **deferred to manual verification** (non-core combinations; VWP6 manual QA). |
  | 3. Editor view — canvas with zones, selected zone, drop/merge/handle states | **Partially captured** — zones + selected zone in all four preset/mode combinations (`02`, `09`, `11`, `12`) | Drop / merge / handle transient states are **deferred to manual verification**: they require precise interactive pointer sessions that automation could not reliably hold for a screenshot. |
  | 4. Wizard dialog (with live preview) | **Captured** in dark `zoneflow-amber` (`04`) | Other preset/mode combinations are **deferred to manual verification** (non-core combinations; VWP6 manual QA). |
  | 5. Picker dialog | **Captured** in dark `zoneflow-amber` (`07`) | Other preset/mode combinations are **deferred to manual verification** (non-core combinations; VWP6 manual QA). |
  | 6. Settings view (including Appearance controls) | **Captured** in dark `zoneflow-amber` (`05`), light `zoneflow-amber` (`08`), light `caffeine` (`10`) | Dark `caffeine` is **deferred to manual verification** (non-core combination; VWP6 manual QA). |
  | 7. Hotkeys view | **Captured** in dark `zoneflow-amber` (`06`) | Other preset/mode combinations are **deferred to manual verification** (non-core combinations; VWP6 manual QA). |
  | 8. Bottom crop / status bar and toasts | **Not captured as a separate cell** | Status bar is visible in the full editor captures, but no toast was active. Toast state is **deferred to manual verification** (requires an interactive session to trigger). |
  | 200% zoom / reduced motion (manual verification items) | **Not captured** | **Deferred to manual verification**: both require an interactive browser/OS session; see the manual verification section below. |

- After VWP5, capture the identical **core subset** using the identical fixture
  setup above and compare side by side. Record observations; do not claim
  improvement without comparison. Deferred cells remain **NOT YET PERFORMED**
  until an interactive session captures them. **After-captures are NOT YET
  PERFORMED**; only the 13 VWP0 baseline PNGs listed in `fixture.md` exist.

### Manual visual verification

Requires an interactive Windows/browser session. Items include: low-chrome appearance, status-chip distinctness, focus rings in light/dark, keyboard operation, reduced motion, 200% zoom, preset switching visibility, dialog scrim readability, and canvas state legibility. All items are **NOT YET PERFORMED** and must be reported separately from automated results.

## Measurable acceptance criteria

| ID | Criterion | Verification method | Status |
|---|---|---|---|
| VAC1 | `scrim` is a required token in both presets/modes; `--color-scrim: var(--scrim);` present in `@theme inline`; `bg-scrim` is the canonical form (no `bg-black/70`, no `bg-[var(--scrim)]`); stale scrim-exception comment removed/rewritten in `ui/dialog.svelte` | `bun run themes:check`, `bun test`, static grep/inspection | Not yet run |
| VAC2 | No hardcoded styling hex in `components/dialogs/*` except documented tints fallback | `frontend/tests/dialog-tokens.test.ts` | Not yet run |
| VAC3 | Elevation tokens defined only in `app.css`, not in preset JSON | Static inspection + schema test | Not yet run |
| VAC4 | Decorative borders removed from panels/cards/rows; functional borders/focus retained | Screenshot comparison + manual checklist | **NOT YET PERFORMED** |
| VAC5 | Presets visibly distinct in surfaces, accent and canvas | Screenshot comparison + manual checklist | **NOT YET PERFORMED** |
| VAC6 | Manifest swatches derived from the dark-mode token map in order `preset.dark.primary`, `preset.dark.background`, `preset.dark.muted-foreground` | Static inspection + unit test | Not yet run |
| VAC7 | WCAG 2.2 AA contrast across all presets/modes | Contrast checker results recorded | **NOT YET PERFORMED** |
| VAC8 | Visible focus, keyboard operation, reduced motion, 200% zoom | Manual keyboard/visual pass | **NOT YET PERFORMED** |
| VAC9 | Canvas states distinguishable in every preset/mode | Contrast check + manual visual pass | **NOT YET PERFORMED** |
| VAC10 | D1–D6 behavior, dirty/undo, overlay independence unchanged | `bun test`, `cargo test`, manual workflow spot-check | Automated not yet run; manual **NOT YET PERFORMED** |
| VAC11 | Core CI-equivalent gates pass (advisory `cargo audit` and artifact upload intentionally excluded, not claimed run) | VWP6 command list | Not yet run |
| VAC12 | Generated CSS current; docs updated; no false manual claims | `themes:check` + doc review | Not yet run |

## Risks, non-goals and safeguards

### Risks and mitigations

| Risk | Safeguard |
|---|---|
| Removing borders also removes affordances | Keep input/focus/handle/separator borders; replace decorative borders with surface steps + elevation |
| Low-chrome surfaces become indistinguishable | Four-level surface hierarchy with contrast-checked steps; adjust preset surface tokens, not re-add outlines |
| Scrim unreadable in light presets | Required `scrim` token with per-mode values; contrast check + visual inspection |
| Preset changes break AA contrast | Contrast checker gates VWP5; prefer surface-hue changes over text-contrast changes |
| Swatch derivation changes picker layout | Derive from normalized tokens only; keep `swatches` shape and count stable |
| Appearance changes leak into workspace state | Keep `AppearanceSettings` outside loaded-draft block; no store/API imports; verify no dirty/undo |
| Canvas states become too faint | Contrast check + AC9 manual pass; keep tokenized colors |
| Stale generated CSS | `bun run themes:check` in CI and locally; commit regenerated CSS with fixture changes |
| False manual claims | All manual/screenshot items labeled NOT YET PERFORMED until actually executed |

### Non-goals

- No new features, workflow changes, profile import/export, app-picker enhancements or hotkey recording.
- No Rust config/DTO/API changes, no new endpoints, no workspace theme fields.
- No Electron/Tauri, no production Node/Bun runtime (Bun stays build-tool only).
- No user theme import, arbitrary user CSS or remote theme loading.
- No keyboard zone selection during drag, no snap-success feedback (still deferred).
- No dependency upgrades unless specifically justified and documented.
- No changes to native overlay behavior or the `MoveWindow` authority boundary.

## Execution model

- **One work package at a time**, with a reviewable checkpoint (commit + push) per WP, following the repo's direct-to-main workflow. CI runs on push/PR.
- **Single subagent at a time**: code via `opencode/hy4-flash-free`, adversarial audit via `opencode/iceman-free`.
- **Fix → re-audit** until zero CRITICAL/MAJOR/MINOR findings before checkpointing.
- **Automated tests/builds** are run and reported; **manual Windows/browser verification** is reported separately and never claimed as performed when it was not.
- No application code is changed by this planning document; implementation happens in the WPs above.

## References

- [Overlay UX + Theme plan (WP0–WP6 baseline)](zoneflow-overlay-ux-theme-plan.md)
- [Feature preservation (D1–D6)](zoneflow-feature-preservation.md)
- [Rust/Axum/Svelte architecture](zoneflow-rust-axum-svelte-architecture.md)
- [Theme and appearance documentation](../theme-and-appearance.md)
- [Manual verification checklist](../manual-verification-checklist.md)
- Historical design inspiration (never loaded by the app): `docs/design/design-mockup.html`, `docs/design/design-v2.html`, `docs/design/design-v3.html`
- Historical screenshots (not a valid baseline): `inspect-01-default.png` … `inspect-07-bottom-crop.png`
