# ZoneFlow Studio — Product Workflow

**Status:** Planning document. No implementation changes are authorized by this file.
**Date:** 2026-11-21
**Basis:** Four independent source audits + official product research (FancyZones, DisplayFusion, MaxTo, Windows Snap).

---

## 1. Product definition

ZoneFlow Studio is a **small, lightweight Windows utility** that replaces a manual AutoHotkey screen-layout script. It lets a non-technical user:

> Open → See active layout → Create/edit zones → Assign applications → Save → Apply → Switch from tray.

It is **not** a general-purpose window manager. It does not implement triggers, functions, wallpapers, or multi-monitor orchestration. **Multi-monitor support is explicitly out of scope** — ZoneFlow manages exactly one work area (the primary monitor) with named profiles of screen zones and application assignments. This matches the original AHK script, which also used only monitor 1.

### Core concepts — kept strictly distinct

| Concept | Definition | Lifetime |
|---|---|---|
| **Profile** | A saved, named workspace configuration: zones + app assignments + rules. Persisted in `ScreenLayout.json`. | Until deleted |
| **Zone** | A rectangular screen region (percent geometry) with at most one app binding and its rules. Part of a profile. | Part of profile |
| **Active profile** | The profile currently applied to Windows (windows positioned, hotkeys registered). | Runtime state |
| **Editor state (draft)** | The configuration currently being modified. May contain unsaved changes. | In-memory until saved |

**Rule:** Saving, editing, and applying are three separate verbs. The UI must never combine them into one ambiguous action. This is the single most important product decision in this document.

### Verbs (distinct, never merged)

- **Edit** — modify the draft in the editor. No effect on disk or Windows.
- **Save** — persist the draft to the active profile on disk. Does not reposition windows.
- **Apply** — use a profile now: position/launch its apps, register its hotkeys. Does not change the draft.
- **Assign** — bind an application to a zone (an edit operation).
- **Disable** — suspend the active layout without deleting anything (new capability; see §6).

---

## 2. User journeys

### Journey A — First run
1. Open ZoneFlow. The Home view shows the active profile (`Website`, empty by default), monitor/work-area info, and status "No layout applied".
2. The user clicks **New profile**, names it, and is taken to the Layout Editor.
3. The editor shows a large canvas with a starter template (e.g. Duo) or an empty canvas with a "Create your first zone" prompt.
4. The user creates/splits/resizes zones, assigns apps by dragging from the library or using the picker, and configures per-app rules contextually.
5. The user clicks **Save** (draft → profile on disk). The unsaved badge clears.
6. The user clicks **Apply**. Windows are positioned/launched; the Home view now shows this profile as active.

### Journey B — Daily use
1. Open ZoneFlow (or use the tray). Home shows the active profile and its status.
2. Switch profiles from the tray menu (one click, no window needed) — the new profile becomes active.
3. Use global hotkeys (`Ctrl+Shift+1..9`) to launch/focus apps by zone index; `Ctrl+Alt+arrows` to move the active window between zones; `Ctrl+drag` a title bar to snap into a zone.
4. If the layout is in the way, use **Disable layout** from the tray; re-enable from the same menu.

### Journey C — Editing an existing profile
1. Select a profile in Home or the rail, click **Edit**.
2. The editor opens with that profile's zones loaded as a draft.
3. Changes are tracked; **Undo/Redo** works; the unsaved badge appears.
4. **Save** persists; **Revert** discards the draft and reloads from disk; **Apply** uses the saved profile.
5. Closing with unsaved changes prompts (Minimize to tray / Exit / Cancel + Remember choice) — existing behavior preserved.

### Journey D — Tray-only operation
The tray must remain useful without opening the main window:
- Show the active profile name (menu header, checkmark next to active item).
- List saved profiles → click to apply.
- **Disable layout** / **Enable layout** toggle.
- **Open editor** → opens the main window on the Layout Editor.
- **Settings…**, **Exit**.

---

## 3. Research: how comparable tools handle these workflows

*Researched behavior (with sources) is separated from recommendations for ZoneFlow.*

### Researched — FancyZones (Microsoft PowerToys)
- A **layout** is zone geometry on a monitor; there is **no declarative app→zone rule** model. App placement is a transient drag plus "move newly created windows to last known zone". Forcing apps into zones is still an open feature request (#11833). [Microsoft Learn FancyZones; PowerToys GitHub README; issue #11833]
- Quick-switch with `Win+Ctrl+Alt+[number]`; mouse-wheel-while-dragging cycles layouts; a layout-name indicator flashes on switch. *(FancyZones does these per-monitor; ZoneFlow will apply them to its single work area only.)* [Microsoft Learn FancyZones]
- Editor has two models: **Grid** (subtractive: split/merge/move dividers) and **Canvas** (additive: add/move/resize zones). Keyboard editing supported. [Microsoft Learn FancyZones]
- Resilience settings: keep windows in zones on resolution change; remap windows by zone number when layout changes; restore original size on unsnap. [Microsoft Learn FancyZones]

### Researched — DisplayFusion
- Multiple profile types (Monitor, Wallpaper, Desktop Icon, Window Position) with consistent **Save / Load / Rename / Delete** vocabulary. [DisplayFusion Help]
- **Window Position Profiles** capture current window positions with match conditions (Process Filename, Window Class, Window Text); documented limitation: they do **not launch applications**, only move already-open windows. [DisplayFusion Help]
- Rich tray menu: profile submenus switch profiles in one click without opening the main window. [DisplayFusion Help]
- App-specific placement via Triggers + Functions, not via the layout itself. [DisplayFusion Help]

### Researched — MaxTo
- **Presets** (geometry, bound to a key) are deliberately separate from **Remembering** (per-app region) and **Recall** (replay) and **Fit to grid** (one-shot, stores nothing). *(MaxTo scopes presets per monitor; ZoneFlow has one work area.)* [MaxTo docs]
- Remembering stores app + optional window type + region as a **fraction of the screen**; first-match-wins list with specificity ordering. [MaxTo docs]
- "MaxTo never fights you: a window is moved once, when it opens — after that, it's yours." [MaxTo docs]
- Live grid overlay with drag-to-select, live resize preview, Esc to cancel; modifier keys signal intent (Ctrl = save preset, Shift = remember). [MaxTo docs]

### Recommendations for ZoneFlow (derived, not copied)
1. Keep **Profile / Zone / Active / Draft** separate (§1) — this is the cleanest part of MaxTo/DisplayFusion and directly satisfies the "don't mix save/edit/apply" requirement.
2. Use **explicit verbs** in UI: Save, Apply, Assign, Disable — never a single "Save & Apply" button. (Current code has Save & Apply; this must be split.)
3. Tray-first profile switching with a checkmark on the active profile (DisplayFusion pattern).
4. Add an explicit **Disable layout** toggle (FancyZones has enable/disable in Settings; a tray toggle is a small, high-value addition).
5. Store geometry as fractions (already done — percent rects) so resolution/DPI changes work.
6. Show a **layout-name indicator** when applying/switching (cheap, high-value feedback).
7. Be honest in UI copy: "Some applications control their own windows and may ignore placement" (cf. FancyZones issue #11833, MaxTo docs).

---

## 4. Main workflows — detailed

### 4.1 Home / Overview
- Active profile name + status (Applied / Not applied / Disabled).
- Monitor/work-area information (primary monitor only, clearly labeled).
- List of saved profiles with **Apply**, **Edit**, **Duplicate**, **Rename**, **Delete** actions.
- Clear saved-vs-unsaved indicator (draft badge belongs to the editor, not Home).
- Buttons: **Open editor**, **Apply**, **Disable layout**.

### 4.2 Layout Editor
- Large visual canvas (the primary content; secondary configuration is contextual).
- Zone operations: create, split, resize (shared-boundary drag), merge (pick-target), delete, draw mode.
- Snap guides at configured points (25/50/75 default).
- Drag-and-drop app assignment from the library; click-to-assign via picker.
- Selection with contextual inspector (appears when a zone/app is selected, not permanently competing).
- Undo/Redo; **Save** and **Revert** clearly separated from **Apply**.
- Empty-state guidance when a profile has no zones.

### 4.3 Application Management
- Library: installed apps (Start Menu + registry), running apps, custom `.exe` via Browse.
- Search; curated list (junk filter already implemented in `rail.rs`).
- Per-app rules: process matching (exe/path), browser URL/title/tab behavior, macro (trigger/keys/delay).
- Assignment list showing app → zone, with priority ordering if multiple rules match.

### 4.4 Profile Management
- Create (name at creation), rename, duplicate, delete (keep ≥1), switch.
- Switching in the editor loads a draft; switching in Home/Tray applies the profile.
- Profile-specific configuration lives in the profile, not globally.

### 4.5 System Tray — complete design
```
ZoneFlow Studio — Active: Website
─────────────────────────────
Profiles              ▸  Website ✓
                         Coding
                         Trading
─────────────────────────────
Disable layout           (toggle → Enable layout)
Open editor…
Settings…
Exit
```
- Header shows active profile; checkmark marks the applied profile.
- Profile submenu applies on click (no main window needed).
- Disable/Enable toggle suspends the active layout without deleting it.
- Open editor opens the main window on the Layout Editor.
- Settings and Exit as today.

---

## 5. Concepts that must not be mixed

| Anti-pattern (current) | Required behavior |
|---|---|
| "Save & Apply" single button | Separate **Save** (persist) and **Apply** (use now) |
| Profile switch in rail only changes selection | Rail switch loads draft; Home/Tray switch applies |
| Launch button only prints to console | Launch must perform real execution (or be renamed "Preview steps") |
| Active profile implicit | Active profile explicitly shown in Home and tray |
| No disable concept | Explicit Disable/Enable layout action |

---

## 6. New capability: Disable layout

Currently absent from both the Rust app and the AHK original. Required by the intended UX.

- Add an `enabled: bool` (or `layout_enabled`) runtime flag — not necessarily persisted; default enabled on startup.
- When disabled: no re-tiling on display change, no auto-launch, drag-to-snap overlay inactive, hotkeys that move windows inactive (profile launch hotkeys may remain or be disabled — decision needed).
- Tray toggle reflects state; Home shows "Layout disabled".
- Re-enabling re-applies the active profile.
- This is a **new** feature, not a preservation item; it must be added without removing existing capabilities.

---

## 7. Open product questions (need user decision before implementation)

1. Should **Apply** launch applications, or only position already-open windows? (DisplayFusion profiles don't launch; current AHK does auto-launch. Recommendation: keep launch, but make it explicit in UI copy.)
2. Should the **Launch** button perform real execution, or be renamed to "Preview steps" if simulation is retained?
3. Should profile switching from the **rail** apply immediately, or only load a draft? (Recommendation: rail = edit draft; Home/Tray = apply.)
4. Should **Disable** also disable global hotkeys, or only window placement?
5. ~~Multi-monitor support~~ — **Decided: NOT in scope.** ZoneFlow targets the primary work area only, matching the original AHK script. No multi-monitor work should be planned or implemented.
