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
