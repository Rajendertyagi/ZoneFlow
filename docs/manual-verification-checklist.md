# Manual verification checklist — ZoneFlow Overlay UX + Theme (WP6)

**Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

This checklist lists every manual acceptance item from the
[ZoneFlow Overlay UX + Theme plan](plans/zoneflow-overlay-ux-theme-plan.md)
("Measurable Acceptance Criteria" and "Verification Plan"). **None of these items
have been performed in WP6.** No box below is checked. Do not interpret the
passing automated checks as evidence for any item on this page — they are
separate and are reported in the WP6 final report.

For each item, an interactive session is required: a Windows desktop for the
native overlay items, and a browser (with the editor served by the built app) for
the browser/accessibility items. Record the date, OS/browser build and observed
result when an item is actually performed, then tick the box and replace the
status note.

---

## Browser / editor (AC1–AC4)

- [ ] **AC1 — Mode and preset persistence.** Set mode (`light`/`dark`/`system`) and a preset; reload, restart the app/browser profile, and inspect `localStorage` keys `mode-watcher-mode` and `mode-watcher-theme`. Values must survive in the same browser profile.
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

- [ ] **AC2 — System mode follows OS theme.** With mode = `system`, change the Windows color theme while the editor is open; the editor must follow the OS light/dark change.
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

- [ ] **AC3 — No theme flash on load.** Hard refresh, first launch and reload in both light and dark OS modes; confirm no incorrect theme flash and that the bootstrap sets `.dark`, `color-scheme` and `data-theme` before paint (inspect initial HTML state before hydration).
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

- [ ] **AC4 — Appearance does not affect workspace state.** Change mode/preset and observe dirty/save/undo indicators; confirm no dirty state, no Save/Discard impact and no undo/redo history entry.
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

## Accessibility / visual (AC7–AC9)

- [ ] **AC7 — WCAG 2.2 AA contrast.** Run a contrast checker/tooling and review every built-in preset and both modes: 4.5:1 normal text, 3:1 large text, 3:1 meaningful non-text UI components and graphical objects (including canvas states and focus rings). Document results per preset/mode.
      **Status: automated calculation COMPLETED** in VWP6 — see
      [visual-contrast-report.md](visual-contrast-report.md). All required pairs
      pass; three exploratory dark-mode `input`-boundary pairs fall just below
      3:1 and are reported as findings. **Visual review of contrast in an
      interactive session is still NOT YET PERFORMED** — this item stays open
      until a human reviews each preset/mode on screen.

- [ ] **AC8 — Keyboard focus, operation and reduced motion.** Keyboard-only pass (Tab/Shift+Tab, arrows, Home/End, Escape, Enter/Space) on modernized primitives; confirm visible focus in light/dark. Enable OS/browser "Reduce motion" / `prefers-reduced-motion` and confirm non-essential motion is reduced/disabled. Also check 200% zoom.
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

- [ ] **AC9 — Canvas and views readable across presets/modes.** Visual QA matrix: canvas grid, zones, handles, previews, guides and labels readable in every built-in preset and both modes; selected/drop/merge/handle states distinguishable.
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

## Visual polish pass (VWP0–VWP6) — VAC4, VAC5, VAC8, VAC9

These items cover the low-chrome visual polish delivered by VWP0–VWP5. They
require an interactive Windows/browser session and are **all NOT YET
PERFORMED**. Automated checks and the automated contrast calculation do **not**
substitute for them.

- [ ] **Low-chrome appearance (VAC4).** In each view (Home, Editor, Settings,
      Hotkeys) confirm decorative boxed borders on panels, cards, chips and rows
      are gone and grouping is conveyed by surface-contrast steps and subtle
      elevation shadows. Confirm functional boundaries remain: input/select
      borders, focus rings, drag/resize handles, meaningful separators, and
      status distinctions (selected / dirty / error / success / disabled).
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

- [ ] **Preset distinctness (VAC5).** Switch between `zoneflow-amber` and
      `caffeine` in both light and dark modes; confirm the change is immediately
      visible in surfaces, accent and canvas (amber/deep-slate vs. warmer
      coffee/cream), and that picker swatches match the active preset.
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

- [ ] **Dialog scrim readability.** Open the wizard, picker, confirm, prompt and
      close dialogs in both presets and both modes; confirm dialog text is
      readable over the scrim in light presets and the scrim is not crushing in
      dark presets. Confirm focus trap, Escape dismiss and focus restore still
      work.
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

- [ ] **Canvas states (VAC9).** In every preset and both modes, confirm the
      canvas grid, zones, handles, previews, guides and labels are readable and
      that selected / drop / merge / handle states are mutually distinguishable.
      Include the transient drop and merge states (not captured in the VWP0
      baseline).
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

- [ ] **Visible focus and keyboard operation (VAC8).** Keyboard-only pass
      (Tab/Shift+Tab, arrows, Home/End, Escape, Enter/Space) on modernized
      primitives; confirm visible focus rings in both light and dark modes and
      that no focus ring was removed when a border was removed.
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

- [ ] **Reduced motion (VAC8).** Enable OS/browser `prefers-reduced-motion:
      reduce` and confirm non-essential transitions on buttons, tabs, switch and
      resizable are disabled.
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

- [ ] **200% zoom (VAC8).** At 200% browser zoom, confirm low-chrome surfaces
      remain distinguishable and controls remain usable (including the 24 × 24
      CSS-pixel minimum interactive target floor).
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

## Windows native overlay (AC10–AC12)

- [ ] **AC10 — Escape cancels active drag.** `Ctrl`+drag a window to show the overlay, then press Escape: the window must not move. Cover continued `Ctrl`+LeftButton hold after Escape (no restart), release of `Ctrl` or LeftButton, and a subsequent fresh gesture.
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

- [ ] **AC11 — Release outside zone does not move window.** `Ctrl`+drag and release the pointer outside any zone; the window must not move.
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

- [ ] **AC12 — Disabled-layout suppression unchanged.** With layout disabled, `Ctrl`+drag must not show the overlay; an in-flight drag must cancel without a drop. Re-enable and confirm normal behaviour.
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

- [ ] **AC10/overlay visual — Active target emphasis and zone labels.** During `Ctrl`+drag, confirm the hovered zone is clearly emphasized (border/alpha/label) and that the overlay draws a 1-based index badge and short zone name.
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

---

## Official-components migration (O1–O6) — manual items

**Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

This section lists every manual acceptance item from the
[ZoneFlow official-components migration plan](plans/zoneflow-official-components-plan.md)
("Acceptance criteria" AC7–AC11 and the per-WP manual acceptance notes). The
migration replaced the 14 hand-rolled `ui/*` primitives and `a11y-helpers.ts`
with official shadcn-svelte / Bits UI components. **None of these items have
been performed.** Every box below is unchecked. Passing automated gates and
static source assertions are **not** evidence for any item on this page — they
are reported separately in the O6 final report. An interactive session is
required; record the date, OS/browser build and observed result when an item is
actually performed, then tick the box and replace the status note.

### Full browser accessibility pass

- [ ] **Keyboard traversal.** Tab/Shift+Tab through every migrated control
      (Button, Badge, Input, Label, Switch, Native Select, Tabs, Context Menu,
      Dialog, ButtonGroup, ToggleGroup) and confirm focus order is logical and
      no control is unreachable or traps focus.
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

- [ ] **Focus visibility in both modes.** Confirm a clearly visible focus ring
      on every migrated control in both light and dark modes (the official
      `focus-visible:ring-*` rings are overridden at call sites to the repo's
      `focus-visible:outline-2 focus-visible:outline-primary` convention).
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

- [ ] **Escape / dismiss behaviour.** Confirm Escape dismisses dialogs and the
      context menu without trapping focus, and that focus returns to the
      triggering element afterwards.
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

- [ ] **Reduced motion.** Enable OS/browser `prefers-reduced-motion: reduce`
      and confirm non-essential transitions/animations on the official
      components (including `tw-animate-css` overlay animations) are disabled.
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

- [ ] **200% zoom.** At 200% browser zoom, confirm migrated controls remain
      usable and the ≥24 × 24 CSS-pixel minimum interactive target floor is
      preserved (icon buttons use `size="icon-sm"`/`icon-xs"` or `class` overrides).
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

- [ ] **Contrast in all four presets × two modes.** Visually review contrast
      (text, focus rings, canvas states, dialog scrim) in every built-in preset
      in both light and dark modes after the official-component migration.
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

### D1–D6 journey

- [ ] **D1–D6 end-to-end journey.** Open → Home → Edit → Assign → Save → Apply
      → Launch & Arrange → Tray switch → Disable → Enable. Confirm the migration
      did not change any frozen workflow behaviour.
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

### Dialogs, menus, tabs and scrim

- [ ] **Stacked dialogs.** Open wizard → confirm and picker → confirm stacks;
      confirm focus, z-order and dismissal behave correctly with the official
      `Dialog` (no hand-rolled focus stack).
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

- [ ] **Context-menu keyboard.** On the canvas, confirm the context menu opens
      with the `ContextMenu` key and with `Shift+F10`, supports keyboard item
      navigation, and dismisses with Escape.
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

- [ ] **Tabs keyboard navigation.** In Picker, Inspector and Library, confirm
      arrow/Home/End tab switching, disabled-item skipping and focus order work
      via Bits UI `Tabs`.
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

- [ ] **Scrim readability.** Open every dialog in all presets and both modes;
      confirm dialog text is readable over the `bg-scrim` overlay and the scrim
      is not crushing in dark presets.
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

---

## Notes on evidence

- **Automated/static evidence exists** for AC5, AC6, AC10 (state-machine tests only), AC12 (test portion only) and AC13 — see the WP6 final report and
  [theme-and-appearance.md](theme-and-appearance.md#ac7--ac8-automatedstatic-evidence-vs-manual-verification). That evidence does **not** substitute for the manual items above.
- **Automated contrast calculation was run in VWP6** (see
  [visual-contrast-report.md](visual-contrast-report.md)). It establishes the
  measured token-pair ratios, not visual contrast compliance. Three exploratory
  dark-mode `input`-boundary pairs fall below 3:1 and are reported as findings.
- **After-images were captured in VWP6** for 12 of the 13 VWP0 baseline cells
  (see the VWP6 status note in
  [the visual polish plan](plans/zoneflow-visual-polish-plan.md)). Screenshot
  capture and side-by-side observation are **not** a substitute for an
  interactive manual visual/keyboard pass.
- **No manual item on this page has been performed.** Every checkbox above
  remains unticked.
- Deferred limitations (resizable keyboard resize, DOM-level component tests, dialog `inert`/scroll-lock, native overlay label legibility, snap-success feedback, keyboard zone selection) are documented in
  [theme-and-appearance.md](theme-and-appearance.md#explicitly-deferred--known-limitations).

---

## Visual implementation (V1–V5) — manual items

**Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

This section lists every manual acceptance item from the
`C:/Users/RTPC/.opencode/plan/zoneflow-visual-implementation-plan.md`
("V5 — Final verification and documentation" and acceptance criteria VAC1–VAC10).
The visual implementation moved both toolbars into the Editor's middle column,
showed both sidebars by default, made the Editor top bar close-only, relocated
`connected` to the Editor status bar, added per-zone geometry info to zone cards,
and replaced decorative borders with surface/shadow separation. **None of these
items have been performed.** Every box below is unchecked. Passing automated
gates and static source assertions are **not** evidence for any item on this
page — they are reported separately in the V5 final report. An interactive
session is required; record the date, OS/browser build and observed result when
an item is actually performed, then tick the box and replace the status note.

### Layout and chrome

- [ ] **Three-pane layout at 1440px.** At a 1440px-wide viewport, confirm the
      Editor shows left rail | middle column (both tool rows + ApplyResults +
      canvas) | inspector, with the tool rows attached to the canvas and no
      layout shift when a zone is selected.
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

- [ ] **Narrow widths.** Below `lg` confirm the left rail is hidden and the
      compact profile selector works; below `xl` confirm the inspector is
      hidden; confirm the Inspector toggle still hides/shows the pane and the
      canvas aspect ratio is unchanged.
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

- [ ] **Top-bar context switching.** In Editor confirm the top-right shows only
      the close button and no connection badge; in Home/Hotkeys/Settings confirm
      all chips remain including the connection badge.
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

- [ ] **Status bar content.** In Editor confirm the status bar shows zones, work
      area, grid (`cols×rows`), selected, layout, the duplicates warning when
      present, and a right-aligned `connected` indicator that reflects the live
      connection state; confirm the apps list is gone.
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

- [ ] **Zone-card info in all four presets × two modes.** On sufficiently large
      zones confirm px size, percentage, x/y and grid cells are readable and do
      not overlap the label/exe, in every built-in preset and both light and
      dark modes; confirm small zones hide the info and drag/resize/right-click
      are unaffected.
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

- [ ] **Shadow separation.** Confirm panes read as distinct via surface steps
      and directional shadows (top bar, tool rows, rail, inspector, status bar,
      inset ring) in all presets × modes, with no lost focus rings and no
      contrast regression.
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

### Accessibility and journey

- [ ] **Keyboard traversal.** Tab/Shift+Tab through the restructured Editor and
      confirm focus order is logical and no control is unreachable or traps
      focus.
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

- [ ] **Focus visibility.** Confirm a clearly visible focus ring in both light
      and dark modes and that no focus ring was removed when a border was
      replaced by a shadow.
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

- [ ] **Reduced motion.** Enable OS/browser `prefers-reduced-motion: reduce` and
      confirm non-essential transitions are disabled.
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

- [ ] **200% zoom.** At 200% browser zoom confirm the three-pane layout, status
      bar and zone-card info remain usable and the ≥24 × 24 CSS-pixel minimum
      interactive target floor is preserved, with no layout shift.
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

- [ ] **D1–D6 journey.** Open → Home → Edit → Assign → Save → Apply → Launch &
      Arrange → Tray switch → Disable → Enable; confirm the visual changes did
      not alter any frozen workflow behaviour.
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

---

## UI polish fix (P0b–P7) — manual items

**Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

This section lists every manual acceptance item from the
[ZoneFlow UI Polish Fix plan](C:/Users/RTPC/.opencode/plan/zoneflow-ui-polish-fix-plan.md)
(PAC0b–PAC6). The UI polish fix removed all status chips from the top bar on
every view, rounded and re-styled the zone-info readout, consolidated the
inspector empty state, moved the toolbar warning/badge into a slim banner,
increased app-list density, constrained canvas framing, and applied the type
scale using existing semantic tokens. **None of these items have been
performed.** Every box below is unchecked. Passing automated gates and static
source assertions are **not** evidence for any item on this page — they are
reported separately in the P7 status record. An interactive session is
required; record the date, OS/browser build and observed result when an item is
actually performed, then tick the box and replace the status note.

### Layout and chrome

- [ ] **Top bar: brand + nav + close only on all pages (PAC0b).** On Home,
      Editor, Hotkeys and Settings confirm the top bar shows only the
      `ZoneFlow Studio` brand, the four nav buttons and the close button — no
      `applied` / `selected` / `saved` / `layout` / `connected` status chips on
      any view. Confirm the information is still available where it belongs
      (Home profile card, Editor status bar, rail badges).
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

- [ ] **Zone info rounded and legible (PAC1).** On sufficiently large zones
      confirm the px / % / x,y / cells readout shows rounded values (no raw
      floats such as `x 0.11560693383216858`), sits on the 12px type step with
      tabular numerics, is right-aligned, has a translucent backing, and is not
      clipped by the zone edge. Confirm small zones still hide the info and
      drag/resize/right-click are unaffected.
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

- [ ] **Inspector single empty state (PAC2).** With no zone selected confirm
      the inspector shows exactly one structured, vertically centred block
      (MousePointer2 icon, `No zone selected` heading, one short guidance line)
      and no duplicate top line or CTA button. Select a zone and confirm the
      tabs populate normally.
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

- [ ] **Toolbar banner (PAC3).** In the Editor confirm the
      `Save or revert your changes before applying.` warning and the `unsaved`
      badge appear together in one slim banner directly under the action bar,
      do not wrap, and do not crowd the buttons. Confirm button groups use 8px
      internal / 16px between-group spacing and ≥24 × 24 targets with visible
      focus rings.
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

- [ ] **App list density (PAC4).** In the left rail confirm app rows are 36px
      tall with a 14px primary label and a 12px dim exe on its own line, long
      names wrap to at most two lines, the leading grip is aligned at a fixed
      left edge, the section tabs/search stay at the top, and there is a single
      scroll container. Confirm drag-to-zone is unchanged.
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

- [ ] **Canvas framing at 1440×900 and 1024×768 (PAC5).** At both viewports
      confirm the work area fills its frame deliberately (no large empty void
      above/below) while the SVG keeps its `viewBox` and
      `preserveAspectRatio="xMidYMid meet"` — the work area must not be
      stretched or its aspect ratio changed. Confirm the inspector hides below
      `xl` and the rail stays.
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

- [ ] **Typography in dark and light (PAC6).** In both modes confirm the type
      scale reads correctly (12px metadata, 14px body/secondary, 16px primary,
      20px section heading; two weights 400/600) and muted text is legible via
      the existing `--zf-faint` / `--zf-muted` tokens. Confirm no off-scale
      sizes and no contrast regression.
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

### Accessibility and journey

- [ ] **Keyboard traversal, focus, reduced motion and 200% zoom.** Keyboard-only
      pass (Tab/Shift+Tab, arrows, Home/End, Escape, Enter/Space) across every
      changed view; confirm logical focus order, visible focus rings in both
      modes, `prefers-reduced-motion: reduce` disables non-essential motion, and
      200% zoom keeps controls usable with ≥24 × 24 targets and no layout shift.
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

- [ ] **D1–D6 journey.** Open → Home → Edit → Assign → Save → Apply → Launch &
      Arrange → Tray switch → Disable → Enable; confirm the UI polish changes
      did not alter any frozen workflow behaviour.
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

---

## Visual audit 2026-11-28 (theme debt T1/T2) — manual re-verification

**Status: NOT YET PERFORMED — requires interactive Windows/browser session.**

The 2026-11-28 visual audit (see
[visual-contrast-report.md](visual-contrast-report.md#visual-audit-2026-11-28--acceptable-exceptions))
covered Editor (empty + zone selected), Home, Hotkeys and Settings; the Layout
Wizard and Assign-app dialogs; ZoneFlow Amber and Caffeine in both dark and
light modes; and viewports 1440×900, 1024×768 and 800×600. That audit was an
observation pass only — it did not re-verify any fix.

- [ ] **T1 badge-fix re-verification.** Re-run the contrast scan in all four
      preset × mode combinations and confirm ≥4.5:1 for the `selected` and
      `zone N` badges after the T1 `bg-transparent` fix.
      **Status: NOT YET PERFORMED — requires interactive Windows/browser session.**
