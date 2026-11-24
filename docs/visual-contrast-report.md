# Visual polish — WCAG 2.2 AA contrast report (VWP6)

**Status: AUTOMATED CALCULATION COMPLETED. This is NOT visual verification.**

This report records an **automated** WCAG 2.2 contrast-ratio calculation over the
token pairs of both built-in presets in both modes. It was produced by a
standalone script that reads the two preset JSON fixtures (the source of truth)
and applies the WCAG relative-luminance / contrast-ratio formula. No browser,
screenshot, or human visual assessment was used to produce these numbers.

- **Date**: 2026-11-24
- **Baseline commit**: `a239def` (VWP5 — preset distinctness and derived swatches)
- **Inputs**: `frontend/src/themes/presets/zoneflow-amber.json`,
  `frontend/src/themes/presets/caffeine.json` (`cssVars.light` / `cssVars.dark`)
- **Thresholds**: normal text **4.5:1**; large text **3:1**; non-text UI /
  graphical objects **3:1** (WCAG 1.4.11). All text pairs were checked at the
  stricter 4.5:1 normal-text threshold; large text would need only 3:1.
- **Method**: sRGB relative luminance with the WCAG linearization
  (`c <= 0.04045 ? c/12.92 : ((c+0.055)/1.055)^2.4`), ratio
  `(L_lighter + 0.05) / (L_darker + 0.05)`. Ratios are rounded to two decimals.

## Result summary

| Category | Checks | Below threshold | Verdict |
|---|---:|---:|---|
| Required text pairs (4.5:1) | 56 (14 × 4 combos) | 0 | **All pass** |
| Required non-text UI pairs (3:1) | 52 (13 × 4 combos) | 0 | **All pass** |
| Exploratory component-boundary pairs (3:1) | 12 (3 × 4 combos) | 3 | **3 findings** (see below) |
| Decorative low-chrome hairlines (documented exceptions) | 16 (4 × 4 combos) | 16 | **Intentional** — not hidden |

**All pairs in the plan's required minimum list pass in every preset/mode.**
Three additional exploratory boundary-token pairs fall just below 3:1 in dark
mode; these are reported as findings, not silently dropped. The decorative
hairlines are intentionally below 3:1 by design and are documented explicitly.

---

## zoneflow-amber / light

Scrim: `rgba(7, 11, 20, 0.55)`.

### Text pairs (required 4.5:1)

| Pair | Ratio | Result |
|---|---:|---|
| `foreground` / `background` | 16.70:1 | PASS |
| `card-foreground` / `card` | 17.74:1 | PASS |
| `popover-foreground` / `popover` | 17.74:1 | PASS |
| `primary-foreground` / `primary` | 5.02:1 | PASS |
| `secondary-foreground` / `secondary` | 13.06:1 | PASS |
| `muted-foreground` / `muted` | 6.92:1 | PASS |
| `muted-foreground` / `background` | 7.14:1 | PASS |
| `muted-foreground-faint` / `muted` | 4.95:1 | PASS |
| `muted-foreground-faint` / `background` | 5.11:1 | PASS |
| `accent-foreground` / `accent` | 8.09:1 | PASS |
| `destructive-foreground` / `destructive` | 4.83:1 | PASS |
| `success-foreground` / `success` | 5.48:1 | PASS |
| `warning-foreground` / `warning` | 4.92:1 | PASS |
| `canvas-label-fg` / `canvas-label-bg` | 7.09:1 | PASS |

### Non-text UI pairs (required 3:1)

| Pair | Ratio | Result |
|---|---:|---|
| `ring` / `background` | 4.73:1 | PASS |
| `ring` / `card` | 5.02:1 | PASS |
| `primary` / `background` | 4.73:1 | PASS |
| `destructive` / `background` | 4.55:1 | PASS |
| `success` / `background` | 5.16:1 | PASS |
| `warning` / `background` | 4.64:1 | PASS |
| `canvas-zone-stroke` / `canvas-bg` | 3.16:1 | PASS |
| `canvas-zone-selected` / `canvas-bg` | 4.73:1 | PASS |
| `canvas-zone-drop` / `canvas-bg` | 4.73:1 | PASS |
| `canvas-zone-merge` / `canvas-bg` | 5.37:1 | PASS |
| `canvas-handle-shared` / `canvas-bg` | 4.73:1 | PASS |
| `canvas-handle-outer` / `canvas-bg` | 4.87:1 | PASS |
| `canvas-preview` / `canvas-bg` | 3.75:1 | PASS |

### Exploratory boundary pairs (3:1 target)

| Pair | Ratio | Result |
|---|---:|---|
| `input` / `background` | 3.16:1 | PASS |
| `input` / `card` | 3.35:1 | PASS |
| `input` / `popover` | 3.35:1 | PASS |

### Decorative hairlines (intentional low-chrome exceptions)

| Pair | Ratio | Note |
|---|---:|---|
| `canvas-grid` / `canvas-bg` | 1.27:1 | Intentional decorative grid hairline |
| `border` / `background` | 1.27:1 | Intentional low-chrome hairline |
| `border` / `card` | 1.35:1 | Intentional low-chrome hairline |
| `border` / `popover` | 1.35:1 | Intentional low-chrome hairline |

---

## zoneflow-amber / dark

Scrim: `rgba(0, 0, 0, 0.72)`.

### Text pairs (required 4.5:1)

| Pair | Ratio | Result |
|---|---:|---|
| `foreground` / `background` | 16.70:1 | PASS |
| `card-foreground` / `card` | 15.19:1 | PASS |
| `popover-foreground` / `popover` | 13.96:1 | PASS |
| `primary-foreground` / `primary` | 8.64:1 | PASS |
| `secondary-foreground` / `secondary` | 12.38:1 | PASS |
| `muted-foreground` / `muted` | 7.06:1 | PASS |
| `muted-foreground` / `background` | 7.77:1 | PASS |
| `muted-foreground-faint` / `muted` | 5.24:1 | PASS |
| `muted-foreground-faint` / `background` | 5.76:1 | PASS |
| `accent-foreground` / `accent` | 12.38:1 | PASS |
| `destructive-foreground` / `destructive` | 6.71:1 | PASS |
| `success-foreground` / `success` | 7.68:1 | PASS |
| `warning-foreground` / `warning` | 11.11:1 | PASS |
| `canvas-label-fg` / `canvas-label-bg` | 11.02:1 | PASS |

### Non-text UI pairs (required 3:1)

| Pair | Ratio | Result |
|---|---:|---|
| `ring` / `background` | 9.16:1 | PASS |
| `ring` / `card` | 8.33:1 | PASS |
| `primary` / `background` | 9.16:1 | PASS |
| `destructive` / `background` | 7.12:1 | PASS |
| `success` / `background` | 10.24:1 | PASS |
| `warning` / `background` | 11.79:1 | PASS |
| `canvas-zone-stroke` / `canvas-bg` | 3.08:1 | PASS |
| `canvas-zone-selected` / `canvas-bg` | 9.16:1 | PASS |
| `canvas-zone-drop` / `canvas-bg` | 9.16:1 | PASS |
| `canvas-zone-merge` / `canvas-bg` | 8.18:1 | PASS |
| `canvas-handle-shared` / `canvas-bg` | 9.16:1 | PASS |
| `canvas-handle-outer` / `canvas-bg` | 8.15:1 | PASS |
| `canvas-preview` / `canvas-bg` | 11.79:1 | PASS |

### Exploratory boundary pairs (3:1 target)

| Pair | Ratio | Result |
|---|---:|---|
| `input` / `background` | 3.08:1 | PASS |
| `input` / `card` | **2.80:1** | **BELOW 3:1 — finding** |
| `input` / `popover` | **2.57:1** | **BELOW 3:1 — finding** |

### Decorative hairlines (intentional low-chrome exceptions)

| Pair | Ratio | Note |
|---|---:|---|
| `canvas-grid` / `canvas-bg` | 1.60:1 | Intentional decorative grid hairline |
| `border` / `background` | 1.60:1 | Intentional low-chrome hairline |
| `border` / `card` | 1.46:1 | Intentional low-chrome hairline |
| `border` / `popover` | 1.34:1 | Intentional low-chrome hairline |

---

## caffeine / light

Scrim: `rgba(41, 33, 26, 0.58)`.

### Text pairs (required 4.5:1)

| Pair | Ratio | Result |
|---|---:|---|
| `foreground` / `background` | 13.78:1 | PASS |
| `card-foreground` / `card` | 15.14:1 | PASS |
| `popover-foreground` / `popover` | 14.68:1 | PASS |
| `primary-foreground` / `primary` | 5.41:1 | PASS |
| `secondary-foreground` / `secondary` | 10.11:1 | PASS |
| `muted-foreground` / `muted` | 5.25:1 | PASS |
| `muted-foreground` / `background` | 5.62:1 | PASS |
| `muted-foreground-faint` / `muted` | 4.70:1 | PASS |
| `muted-foreground-faint` / `background` | 5.03:1 | PASS |
| `accent-foreground` / `accent` | 7.61:1 | PASS |
| `destructive-foreground` / `destructive` | 4.83:1 | PASS |
| `success-foreground` / `success` | 5.02:1 | PASS |
| `warning-foreground` / `warning` | 5.02:1 | PASS |
| `canvas-label-fg` / `canvas-label-bg` | 7.22:1 | PASS |

### Non-text UI pairs (required 3:1)

| Pair | Ratio | Result |
|---|---:|---|
| `ring` / `background` | 5.00:1 | PASS |
| `ring` / `card` | 5.49:1 | PASS |
| `primary` / `background` | 5.00:1 | PASS |
| `destructive` / `background` | 4.23:1 | PASS |
| `success` / `background` | 4.39:1 | PASS |
| `warning` / `background` | 4.40:1 | PASS |
| `canvas-zone-stroke` / `canvas-bg` | 3.19:1 | PASS |
| `canvas-zone-selected` / `canvas-bg` | 5.00:1 | PASS |
| `canvas-zone-drop` / `canvas-bg` | 5.00:1 | PASS |
| `canvas-zone-merge` / `canvas-bg` | 4.75:1 | PASS |
| `canvas-handle-shared` / `canvas-bg` | 5.00:1 | PASS |
| `canvas-handle-outer` / `canvas-bg` | 4.40:1 | PASS |
| `canvas-preview` / `canvas-bg` | 3.49:1 | PASS |

### Exploratory boundary pairs (3:1 target)

| Pair | Ratio | Result |
|---|---:|---|
| `input` / `background` | 3.19:1 | PASS |
| `input` / `card` | 3.50:1 | PASS |
| `input` / `popover` | 3.40:1 | PASS |

### Decorative hairlines (intentional low-chrome exceptions)

| Pair | Ratio | Note |
|---|---:|---|
| `canvas-grid` / `canvas-bg` | 1.36:1 | Intentional decorative grid hairline |
| `border` / `background` | 1.36:1 | Intentional low-chrome hairline |
| `border` / `card` | 1.50:1 | Intentional low-chrome hairline |
| `border` / `popover` | 1.45:1 | Intentional low-chrome hairline |

---

## caffeine / dark

Scrim: `rgba(0, 0, 0, 0.72)`.

### Text pairs (required 4.5:1)

| Pair | Ratio | Result |
|---|---:|---|
| `foreground` / `background` | 16.62:1 | PASS |
| `card-foreground` / `card` | 14.15:1 | PASS |
| `popover-foreground` / `popover` | 12.66:1 | PASS |
| `primary-foreground` / `primary` | 7.67:1 | PASS |
| `secondary-foreground` / `secondary` | 10.43:1 | PASS |
| `muted-foreground` / `muted` | 6.14:1 | PASS |
| `muted-foreground` / `background` | 7.21:1 | PASS |
| `muted-foreground-faint` / `muted` | 4.82:1 | PASS |
| `muted-foreground-faint` / `background` | 5.66:1 | PASS |
| `accent-foreground` / `accent` | 10.31:1 | PASS |
| `destructive-foreground` / `destructive` | 5.97:1 | PASS |
| `success-foreground` / `success` | 5.23:1 | PASS |
| `warning-foreground` / `warning` | 9.89:1 | PASS |
| `canvas-label-fg` / `canvas-label-bg` | 8.90:1 | PASS |

### Non-text UI pairs (required 3:1)

| Pair | Ratio | Result |
|---|---:|---|
| `ring` / `background` | 8.44:1 | PASS |
| `ring` / `card` | 7.19:1 | PASS |
| `primary` / `background` | 8.44:1 | PASS |
| `destructive` / `background` | 6.57:1 | PASS |
| `success` / `background` | 10.44:1 | PASS |
| `warning` / `background` | 10.89:1 | PASS |
| `canvas-zone-stroke` / `canvas-bg` | 3.53:1 | PASS |
| `canvas-zone-selected` / `canvas-bg` | 8.44:1 | PASS |
| `canvas-zone-drop` / `canvas-bg` | 8.44:1 | PASS |
| `canvas-zone-merge` / `canvas-bg` | 6.30:1 | PASS |
| `canvas-handle-shared` / `canvas-bg` | 8.44:1 | PASS |
| `canvas-handle-outer` / `canvas-bg` | 10.45:1 | PASS |
| `canvas-preview` / `canvas-bg` | 10.45:1 | PASS |

### Exploratory boundary pairs (3:1 target)

| Pair | Ratio | Result |
|---|---:|---|
| `input` / `background` | 3.53:1 | PASS |
| `input` / `card` | 3.00:1 | PASS (at threshold) |
| `input` / `popover` | **2.69:1** | **BELOW 3:1 — finding** |

### Decorative hairlines (intentional low-chrome exceptions)

| Pair | Ratio | Note |
|---|---:|---|
| `canvas-grid` / `canvas-bg` | 1.61:1 | Intentional decorative grid hairline |
| `border` / `background` | 1.61:1 | Intentional low-chrome hairline |
| `border` / `card` | 1.37:1 | Intentional low-chrome hairline |
| `border` / `popover` | 1.23:1 | Intentional low-chrome hairline |

---

## Findings and honest framing

1. **Required pairs all pass.** Every text pair (4.5:1) and every non-text UI /
   canvas-state pair (3:1) in the plan's minimum list passes in all four
   preset/mode combinations. This closes the AC7 "not established" gap **for
   the measured token pairs only**.

2. **Three exploratory boundary findings (below 3:1).** The `input` token —
   used as the functional boundary on dialog/menu/tooltip panels (`bg-popover`)
   and outline buttons (`bg-zf-surface`) — falls below 3:1 against those
   surfaces in dark mode:
   - `zoneflow-amber` dark: `input`/`card` 2.80:1, `input`/`popover` 2.57:1
   - `caffeine` dark: `input`/`popover` 2.69:1
   
   These are **borderline** (2.57–2.80 vs 3.0 required). They are reported as
   findings for design review. VWP6 does **not** change application code, so
   these token values were left untouched. Note that the low-chrome design
   identifies components primarily through surface contrast and elevation
   shadows rather than the 1px boundary alone; whether these specific boundary
   ratios constitute a WCAG failure depends on that design judgement and on
   manual review. They are **not** claimed as passing.

3. **Decorative hairlines are intentional exceptions.** The `border` token and
   `canvas-grid` token are deliberately low-contrast (1.23–1.61:1) to achieve
   the low-chrome look. They are documented here rather than hidden. They are
   not used as the sole means of identifying a component or state; functional
   boundaries (inputs, focus rings, handles, selected/drop/merge states) use
   the separate `input`, `ring`, and `canvas-*` tokens, all of which pass.

4. **Scrim.** The scrim is a translucent overlay (`rgba(...)`), so its effective
   contrast depends on what is behind it. Dialog text sits on the **opaque**
   `popover` surface above the scrim, so the measured `popover-foreground` /
   `popover` ratios (13.96–17.74:1) govern dialog text legibility, not the
   scrim value itself. Whether the scrim is "dark enough" in light presets and
   "not too crushing" in dark presets is a **visual** judgement and remains a
   manual item.

5. **This is not visual verification.** These are computed ratios from token
   values. They do not account for rendered font sizes/weights, opacity
   compositing, anti-aliasing, actual on-screen surface stacking, or human
   perception. The manual visual pass (AC7/AC8/AC9) remains **NOT YET
   PERFORMED** — see `manual-verification-checklist.md`.
