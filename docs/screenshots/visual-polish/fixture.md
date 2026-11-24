# Visual polish baseline fixture (VWP0)

Baseline captured at commit `c47d625` on 2026-11-24 from the built release app
(`target/release/screenlayout.exe`) serving the editor at `http://127.0.0.1:4000/`.
(`c47d625` is a docs-only commit on top of `ba9b393` — it changed only
`docs/plans/zoneflow-visual-polish-plan.md` — so the UI captured here is unchanged
from the `ba9b393` baseline referenced in the plan.)

**Status: baseline captures completed for the SUBSET of matrix cells listed below; remaining matrix cells are gaps (see Gaps).** The
before-images in this directory are the real baseline. After-images have NOT been
captured yet (that happens after VWP5). No contrast measurement or manual
accessibility pass was performed as part of capture.

## Deterministic fixture setup (use identically for before and after)

1. **Viewport**: browser viewport fixed at **1440 × 900**, zoom 100%. Recorded here.
2. **Profile**: created a profile named **`Visual QA`** via the existing New profile prompt.
3. **Template**: applied the **`Duo`** template via the existing Layout wizard
   (deterministic 2 zones, "Replace the current zones" checked). This one fixed
   template choice is used everywhere.
4. **Apps** (assigned via the existing picker/Browse flow; exact values recorded so
   before/after use identical data):
   - Zone 1 — label **`Editor`**, app **`notepad++.exe`**, path
     `C:\Program Files\Notepad++\notepad++.exe`
   - Zone 2 — label **`Archive`**, app **`7zFM.exe`**, path
     `C:\Program Files\7-Zip\7zFM.exe`
   - No substitutes were needed. If an app is unavailable on another machine,
     record the substitute here and keep it identical across captures.
5. **Selected state**: click zone 1 to select it (deterministic).
6. **Drop state**: drag the recorded app from the library over zone 2 and capture
   while hovering BEFORE release. (Not captured in this baseline batch — see gaps.)
7. **Merge state**: use the existing Merge toolbar action, click zone 1 as source
   then hover zone 2 as target. (Not captured in this baseline batch — see gaps.)
8. **Mode/preset**: set via the existing Appearance controls before each capture.
   Both presets × both modes were captured for the editor fixture zones; Home was
   captured in dark mode for both presets; Settings was captured for both presets
   in light mode and amber in dark mode.
9. The draft was **Saved** after fixture setup so the baseline state is clean
   (`saved` in the top bar). Appearance was restored to **System / ZoneFlow Amber**
   after capture so the browser profile was left in its original preference state.

## Captured cells

| File | View | Mode | Preset |
|---|---|---|---|
| `01-editor-dark-amber.png` | Editor (initial, 2 unassigned zones) | dark | zoneflow-amber |
| `02-editor-zones-dark-amber.png` | Editor (fixture zones, inspector closed) | dark | zoneflow-amber |
| `03-home-dark-amber.png` | Home | dark | zoneflow-amber |
| `04-wizard-dark-amber.png` | Layout wizard dialog (Duo selected) | dark | zoneflow-amber |
| `05-settings-dark-amber.png` | Settings (Appearance visible) | dark | zoneflow-amber |
| `06-hotkeys-dark-amber.png` | Hotkeys | dark | zoneflow-amber |
| `07-picker-dark-amber.png` | Assign application dialog | dark | zoneflow-amber |
| `08-settings-light-amber.png` | Settings | light | zoneflow-amber |
| `09-editor-zones-light-amber.png` | Editor (fixture zones) | light | zoneflow-amber |
| `10-settings-light-caffeine.png` | Settings | light | caffeine |
| `11-editor-zones-light-caffeine.png` | Editor (fixture zones) | light | caffeine |
| `12-editor-zones-dark-caffeine.png` | Editor (fixture zones) | dark | caffeine |
| `13-home-dark-caffeine.png` | Home | dark | caffeine |

## Capture metadata (text evidence)

This section records the text-based facts that were true at capture time so a
reviewer who cannot view the PNGs can cross-check each image against its recorded
state. The view, mode and preset columns are taken directly from the **Captured
cells** table above. The DOM/theme columns follow deterministically from the
recorded mode and preset via the pre-paint bootstrap in `frontend/index.html`
(lines 41–44): the bootstrap toggles the `dark` class on `<html>` when the resolved
mode is `dark`, sets `style.colorScheme` to the resolved mode, and sets
`data-theme` to the preset id. No DOM/theme value below is inferred beyond that
documented mapping.

| File | View | Mode | Preset id | `<html>` class | `data-theme` | `color-scheme` |
|---|---|---|---|---|---|---|
| `01-editor-dark-amber.png` | Editor (initial, 2 unassigned zones) | dark | `zoneflow-amber` | contains `dark` | `zoneflow-amber` | `dark` |
| `02-editor-zones-dark-amber.png` | Editor (fixture zones, inspector closed) | dark | `zoneflow-amber` | contains `dark` | `zoneflow-amber` | `dark` |
| `03-home-dark-amber.png` | Home | dark | `zoneflow-amber` | contains `dark` | `zoneflow-amber` | `dark` |
| `04-wizard-dark-amber.png` | Layout wizard dialog (Duo selected) | dark | `zoneflow-amber` | contains `dark` | `zoneflow-amber` | `dark` |
| `05-settings-dark-amber.png` | Settings (Appearance visible) | dark | `zoneflow-amber` | contains `dark` | `zoneflow-amber` | `dark` |
| `06-hotkeys-dark-amber.png` | Hotkeys | dark | `zoneflow-amber` | contains `dark` | `zoneflow-amber` | `dark` |
| `07-picker-dark-amber.png` | Assign application dialog | dark | `zoneflow-amber` | contains `dark` | `zoneflow-amber` | `dark` |
| `08-settings-light-amber.png` | Settings | light | `zoneflow-amber` | no `dark` class | `zoneflow-amber` | `light` |
| `09-editor-zones-light-amber.png` | Editor (fixture zones) | light | `zoneflow-amber` | no `dark` class | `zoneflow-amber` | `light` |
| `10-settings-light-caffeine.png` | Settings | light | `caffeine` | no `dark` class | `caffeine` | `light` |
| `11-editor-zones-light-caffeine.png` | Editor (fixture zones) | light | `caffeine` | no `dark` class | `caffeine` | `light` |
| `12-editor-zones-dark-caffeine.png` | Editor (fixture zones) | dark | `caffeine` | contains `dark` | `caffeine` | `dark` |
| `13-home-dark-caffeine.png` | Home | dark | `caffeine` | contains `dark` | `caffeine` | `dark` |

### Fixture facts true at capture time

These come from the deterministic fixture setup recorded above and were true for
the batch (the setup was not changed between captures):

- **Profile name**: `Visual QA` (created via the existing New profile prompt).
- **Zone 1**: label `Editor`, app `notepad++.exe`
  (`C:\Program Files\Notepad++\notepad++.exe`).
- **Zone 2**: label `Archive`, app `7zFM.exe`
  (`C:\Program Files\7-Zip\7zFM.exe`).
  These zone labels/apps apply to the "fixture zones" editor captures
  (`02`, `09`, `11`, `12`); `01` is the initial state with 2 unassigned zones.
- **Top bar**: the four status chips (applied / selected / saved / layout) were
  present. After the fixture was Saved, the saved chip read `saved`.
- **Viewport**: 1440 × 900, zoom 100%.
- **Selected state**: zone 1 was selected for the fixture-zone editor captures.
- After the batch, Appearance was restored to **System / ZoneFlow Amber**; this
  was not part of any captured image.

## Token decisions confirmed

The five locked decisions in `docs/plans/zoneflow-visual-polish-plan.md`
("Locked decisions (resolved — not open questions)") were reviewed against the
captured baseline and were **unchanged at capture time** on 2026-11-24 at commit
`c47d625`:

1. **Scrim token — ADD** (required `scrim` key, `bg-scrim` as the canonical
   utility, `--color-scrim: var(--scrim);` in `@theme inline`).
2. **Elevation/shadow tokens — define in `app.css` only** (dark in both modes,
   not derived from `--foreground`, not in preset JSON).
3. **Preset distinctness — meaningfully but conservatively distinct**
   (`zoneflow-amber` amber/deep-slate; `caffeine` warmer coffee/cream; AA contrast
   preserved).
4. **Manifest swatches — DERIVE** from the dark-mode token map in the order
   `preset.dark.primary`, `preset.dark.background`, `preset.dark.muted-foreground`.
5. **Documented exceptions remain** (`zone_tints` defaults and test-fixture hex
   kept as documented exceptions).

No token decision was revised as a result of the VWP0 capture.

## Gaps (honest)

- **Drop and merge transient states were not captured** in this baseline batch.
  They require precise pointer interactions that the automation could not reliably
  hold for a screenshot. They remain **NOT YET PERFORMED** and must be captured
  manually before a valid before/after comparison for those states can be claimed.
- **Bottom crop / status bar and toasts** were not captured as a separate cell;
  the status bar is visible in the full editor captures, but no toast was active.
- **200% zoom** and **reduced-motion** states were not captured (manual-only).
- No contrast checker was run; AC7 remains not established by automated evidence.

## Capture procedure (for reproducibility)

The editor tab was driven with browser automation; `paseo_browser_screenshot`
writes PNGs to the Paseo attachments temp directory, from which they were copied
into this directory with the names above. The same procedure must be used for the
after-images: identical viewport, identical fixture, identical mode/preset order.
