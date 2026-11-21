# ZoneFlow Studio — Product Workflow

**Status:** Authoritative for product behavior. Decisions D1–D6 are frozen and remain fully in force under the new `zoneflow-rust-axum-svelte-architecture.md` direction; the new API contracts encode them. No implementation is authorized by this file.
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

**Rule:** Saving, editing, and applying are three separate verbs. The UI must never combine them into one ambiguous action. This is the single most important product decision in this document. *(One documented exception exists: auto-save of a dirty draft on Exit via the existing close dialog — see §2a. It is disclosed in the dialog copy, not hidden.)*

### Verbs (distinct, never merged)

- **Edit** — modify the draft in the editor. No effect on disk or Windows.
- **Save** — persist the draft to the active profile on disk. Does not reposition windows.
- **Apply** — use a profile now: position/launch its apps, register its hotkeys. Does not change the draft.
- **Assign** — bind an application to a zone (an edit operation).
- **Disable** — suspend the active layout without deleting anything (new capability; see §6).

---

## 1a. Data ownership — profile-specific assignments (CONFIRMED requirement)

**Verified against source (2026-11-21):** the current data model **already** stores application assignments per profile. No data-model rewrite is required.

| Data | Owner | Source evidence |
|---|---|---|
| Zone geometry (x/y/w/h %) | `Zone` inside a `Profile` | `config.rs:104-108`, `:150-156` |
| App binding (label, exe, path) | `Zone` inside a `Profile` | `config.rs:86-89`; `attach_app` writes only to `active_mut().zones` at `mod.rs:585-596` |
| Launch mode | `Zone` (per profile) | `config.rs:90` |
| Browser rules (url, title_match, tab_behavior) | `Zone` (per profile) | `config.rs:91-93` |
| Macro (enabled, trigger, keys, delay) | `Zone` (per profile) | `config.rs:94-103` |
| Notes (non-executable) | `Zone` (per profile) | `config.rs:99-102` |
| Profiles map | `AppConfig.profiles: BTreeMap<String, Profile>` | `config.rs:272` |
| Global hotkey bindings | `AppConfig.hotkeys` (global, not per profile) | `config.rs:278` |
| Application library | **Not persisted** — runtime enumeration only | `rail.rs:485-569` (Start Menu / registry / tasklist) |

### What this means

- The same application **can** occupy different zones in different profiles — each profile owns its own `Vec<Zone>`, each zone owns its own exe/path/rules.
- Editing one profile **cannot** modify another — they are separate `Profile` structs in the map; `attach_app` targets `active_mut()` only.
- Browser rules, macros and launch settings are **zone-scoped, therefore profile-scoped**. Their ownership is unambiguous.
- The global library contains **reusable application entries only** (name/exe/path discovered at runtime). It stores no assignments. Assigning an app copies its identity into the target zone; removing an assignment clears the zone fields but leaves the library entry untouched.
- Profile switching loads that profile's own zones and rules (`active_profile` key → `profiles` map).

### Example (the required behavior, already supported by the model)

| Application | Coding | Trading | Gaming |
|---|---|---|---|
| Chrome | Zone 1 | Zone 2 | Zone 3 |
| VS Code | Zone 2 | — | — |
| Discord | Zone 3 | Zone 3 | Zone 1 |
| TradingView | — | Zone 1 | — |

**Remaining work is UI clarity, not data structure:** make assignments visible per profile (e.g. an assignment list/matrix in the editor), and ensure the picker/library is clearly presented as reusable entries rather than assignments.

---

## 2. User journeys

### Journey A — First run
1. Open ZoneFlow. The Home view shows the **selected** profile (`Website`, empty by default), monitor/work-area info, and status "Not applied" (no profile has been applied yet on first run). The selected profile is not the same as an applied profile — see §1 state dimensions.
2. The user clicks **New profile**, names it, and is taken to the Layout Editor.
3. The editor shows a large canvas with a starter template (e.g. Duo) or an empty canvas with a "Create your first zone" prompt.
4. The user creates/splits/resizes zones, assigns apps by dragging from the library or using the picker, and configures per-app rules contextually.
5. The user clicks **Save** (draft → profile on disk). The unsaved badge clears.
6. The user clicks **Apply** to arrange already-running apps, or **Launch & Arrange** to also launch missing apps (D1). The Home view now shows this profile as **Applied**.

### Journey B — Daily use
1. Open ZoneFlow (or use the tray). Home shows the active profile and its status.
2. Switch profiles from the tray menu (one click, no window needed) — the new profile is applied immediately when layout is enabled; when disabled, selection changes only (D6).
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

## 2a. Exact behavior for ambiguous situations

These are **decisions** for the intended workflow. Each must be implemented consistently.

| Situation | Exact behavior |
|---|---|
| **Editing the active profile** | Allowed. The editor works on a draft copy; the applied layout on Windows is unchanged until **Apply**. Home/tray continue to show the active (saved) profile, not the draft. |
| **Saving without applying** | Draft is written to disk for that profile. Unsaved badge clears. Windows are **not** repositioned; active profile status is unchanged. |
| **Applying with unsaved changes** | **(D2 — APPROVED)** **Disable Apply while the draft has unsaved changes.** Show the explanation: "Save or revert your changes before applying." Never silently apply an old saved version when the user expects their edits to be applied. The same rule applies to **Launch & Arrange**. |
| **Switching profiles with unsaved changes** | **(D2 — APPROVED)** Requires explicit **Discard** or **Cancel**. Never silently lose draft changes. Switching loads the target profile's own zones/rules. |
| **Apply failure / partial application** | Apply is best-effort per zone. Each zone reports success/failure in the console. A summary shows "Applied 2/3 zones; 1 failed (app not found)". The **Applied** dimension records the profile that was attempted with status "Applied with errors" (a valid Applied value — see GUI state model); the **Selected** dimension is independent. No silent partial success. |
| **Reverting changes** | Reload the draft from disk for the current profile; clear unsaved badge; reset selection and transient drag/merge state. Re-register hotkeys from the **applied profile's** saved config (not the edited profile's), so reverting an edit never replaces the applied profile's hotkeys. Does not change the applied profile on Windows. |
| **Replacing an application assignment** | **(D3 — APPROVED)** Preserve zone geometry, ID and position. Replace application identity (label, exe, path). Reset application-specific rules (launch mode, URL, title, tab, macro) to defaults. Show confirmation before resetting non-default rules. Do **not** delete the application from the global library. |
| **Removing an assignment** | Clears the zone's app fields (label/exe/path) and rules; the zone remains as an empty region. The application entry stays in the global library. Never deletes the library entry. |
| **Same app in multiple zones within one profile** | See §3b — backend limitation documented; safest initial behavior defined. Different profiles may freely assign the same app to different zones. |
| **Closing with unsaved changes** | Existing close dialog preserved: Minimize to tray / Exit / Cancel + Remember choice. **Exception to verb separation:** if Exit is chosen with unsaved changes, the draft is auto-saved by `on_exit` (existing behavior, `mod.rs:1213-1221`). This is a documented exception — the dialog copy must state "Unsaved changes will be saved on exit." |
| **Apply / Launch & Arrange while layout is Disabled** | Explicit Apply/Launch & Arrange buttons are **inactive while disabled** (D4 suspends all automation). Show "Layout is disabled — enable it to apply" with an **Enable layout** action. Profile selection remains available (select-only). |
| **Save failure (disk write error)** | Report the error in the console and a toast; keep the draft and unsaved badge intact (nothing is silently lost). Do not change the applied profile. |
| **Delete / Rename / Duplicate the active or applied profile** | Allowed. Deleting the applied profile clears the Applied dimension (status "Not applied") and selects another profile; renaming keeps the same profile identity (zones/bindings intact); duplicating creates a separate copy. If the draft is dirty, apply the D2 Discard/Cancel rule before destructive operations. |

---

## 3a. Application assignment workflow — complete interaction

The canvas stays primary; assignment is a deliberate gesture, not a permanent panel.

1. **Drag from library → zone.** The left rail (Apps tab) lists reusable app entries. Dragging one onto a zone calls `attach_app(zone_id, app)` — verified at `mod.rs:585-596` — which writes label/exe/path into that zone of the active profile. Drop-target highlight shows the target zone.
2. **Select zone → picker.** Click a zone, then "Assign app…" opens the picker (Installed / Running / Browse). Selecting an entry performs the same `attach_app` operation.
3. **Replace an existing assignment.** Drop/select a different app on an occupied zone → per §2a (confirm if rules are non-default).
4. **Remove an assignment.** Zone context menu → "Remove app from zone" → clears zone fields only; library entry untouched.
5. **Configure rules contextually.** With a zone selected, the contextual inspector shows Launch mode, Process matching (exe/path), Browser rules (URL/title/tab), and Macro (trigger/keys/delay). These are zone-scoped → profile-scoped (§1a).
6. **Show assignments clearly.** The editor footer "Apps in this workspace" lists each assigned app → zone for the **current profile** (existing `apps_strip`). Optionally an assignment matrix across profiles (§1a example) — a future enhancement, not required initially.

---

## 3b. Same application in multiple zones — backend limitation (verified)

**Different profiles:** freely supported and required. Chrome → Zone 1 in Coding, Zone 2 in Trading, Zone 3 in Gaming (§1a). Each profile owns independent zones; no conflict.

**Within ONE profile:** the current backend has a real limitation.

- `find_window_by_path_min` (`engine.rs:315-334`) returns a **single** `HWND` — the first exe-name match, or the exact full-path match. It has no concept of "already placed" windows and cannot return multiple windows for one app.
- `retile` (`engine.rs:522-534`) iterates zones and calls `find_window_by_path_min` **per zone**. If two zones in one profile specify the same exe/path, **both zones receive the same first HWND** and the window is moved twice — the last zone wins. The same applies to `execute_zone` during Apply/Launch & Arrange.
- **Why repeated placement is incorrect:** one window cannot occupy two zones simultaneously. Silently moving the same window between zones produces unpredictable final placement and misleading "success" logs.

**Safest behavior for the initial implementation:**
1. Detect duplicate exe/path assignments **within one profile** in the editor.
2. Show a clear warning: "Chrome is assigned to Zone 1 and Zone 2. Only one window can be placed per app; the last zone will win." Recommend removing the duplicate or using different apps.
3. Do **not** claim reliable multi-window support. Do not silently move the same window between zones without warning.
4. Keep the model permissive (no forced deletion) — the user may intend two instances of the same app, which the backend cannot currently distinguish.

**Future capability (separate, not in scope):** proper multi-window matching — enumerating all windows for an exe/path, tracking which HWND was already placed, and assigning distinct instances to distinct zones. This requires a new matching/placement design and is explicitly deferred.

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
- Active profile name + **independent** status indicators: Applied (profile name / Not applied) and Enabled (Enabled / Disabled) — never one merged badge (see GUI state model).
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
- **(D6)** Profile submenu **applies immediately on click** when layout is enabled (no main window needed). When disabled, selecting a profile changes the selection only — nothing is applied or launched; status shows "Disabled".
- **(D4)** Disable/Enable toggle suspends all layout automation without deleting anything; tray, profile selection, editor and settings stay available.
- Open editor opens the main window on the Layout Editor.
- Settings and Exit as today.

---

## 5. Concepts that must not be mixed

| Anti-pattern (current) | Required behavior |
|---|---|
| "Save & Apply" single button | Separate **Save** (persist) and **Apply** (use now) |
| Profile switch in rail only changes selection | Rail switch loads draft; Home/Tray switch applies |
| Launch button only prints to console | **(D5 — APPROVED)** Launch performs real execution; a separate Preview action inspects steps without running |
| Active profile implicit | Active profile explicitly shown in Home and tray |
| No disable concept | Explicit Disable/Enable layout action |

---

## 4a. Apply behavior — two modes (D1 — APPROVED)

**Decision D1 is APPROVED. Both modes are required as distinct, understandable actions.**

| Action | Behavior |
|---|---|
| **Apply** (default) | Arrange applications that are **already running**. **Never** unexpectedly launch missing applications. |
| **Launch & Arrange** | Explicitly launch missing assigned applications, then arrange their windows into configured zones. Respect each zone's existing **Launch/Attach** setting. Report individual launch and positioning failures. |

- These are **two separate buttons/actions**, not a mode toggle that silently changes behavior.
- **Apply** is the default and safe action. **Launch & Arrange** is explicit and opt-in.
- Per-zone Launch/Attach: a zone in `Attach` mode must not launch; it only attaches if running (existing `LaunchMode` semantics, `config.rs:90`).
- Failures are reported per zone (e.g. "Chrome: launched, positioned" / "Obsidian: not running, Attach mode — skipped" / "VS Code: launch failed").

**Current source reality:** `save_apply` (`mod.rs:312-327`) only saves + re-tiles + re-registers hotkeys — it does **not** launch apps. Real launching happens via hotkeys (`Action::LaunchApp`) and the opt-in startup auto-launch worker. So today's Apply already behaves as the **arrange-running-only** action. **Launch & Arrange** is a new explicit action built on the existing `execute_zone_with_config` path.

---

## 6. Disable layout (D4 — APPROVED)

Currently absent from both the Rust app and the AHK original. Required by the intended UX. **Decision D4 is APPROVED.**

**Temporary disabling vs removing/deactivating a profile:**
- **Disable** is a temporary, runtime suspension of layout automation. It does **not** delete, deactivate, or modify any profile. The selected profile remains visible in Home/tray, marked "Disabled".
- Removing/deactivating a profile is a separate, destructive management action (Delete) — unaffected by this feature.

**Exact behavior when disabled — what is suspended (all approved):**

| Capability | Suspended? |
|---|---|
| Automatic window positioning (re-tile on Apply/display change) | **Yes** |
| Drag-to-snap overlay | **Yes** |
| Automatic application launching (startup auto-launch) | **Yes** |
| Window movement hotkeys (`Ctrl+Alt+arrows`) | **Yes** |
| Application launch/focus hotkeys (`Ctrl+Shift+1..9`) | **Yes** |

**What stays available while disabled:** tray access, profile selection, editor access, and settings.

**Profile selection while disabled:** changes the **selected profile only** — it must **not** apply or launch anything. Show explicit "Disabled" status.

**What Enable restores:** layout automation and re-applies the selected profile using the normal **Apply** behavior (D1 default — arrange running apps only; no unexpected launches).

**Startup persistence (defined):** the disabled state is **runtime-only and resets to enabled on application restart**. This is predictable and documented: on every launch, layout automation is enabled and the selected profile is applied per normal startup behavior. *(If persistence is later desired, it becomes a separate Settings option — not in scope now.)*

**State visibility:** tray toggle shows "Disable layout" / "Enable layout"; Home shows a clear "Disabled" badge; status bar reflects disabled state.

- Add a `layout_enabled: bool` runtime flag — **not persisted**; default `true` on startup.
- This is a **new** feature, not a preservation item; it must be added without removing existing capabilities.

---

## 7. Decisions (frozen) and remaining technical questions

### APPROVED product decisions (D1–D6 — confirmed requirements, not open questions)
- **D1 — Apply modes:** both **Apply** (arrange running apps only, default) and **Launch & Arrange** (explicitly launch missing apps, respect Launch/Attach, report failures) are required as distinct actions (§4a).
- **D2 — Unsaved changes:** disable Apply while dirty with explanation "Save or revert your changes before applying"; profile switching with unsaved changes requires explicit Discard/Cancel (§2a).
- **D3 — Replace assignment:** preserve zone geometry/ID/position; replace app identity; reset rules to defaults with confirmation for non-default rules; never delete the library entry (§2a).
- **D4 — Disable:** suspends all window positioning, drag-to-snap, auto-launch, window-movement hotkeys and launch/focus hotkeys; tray/profile/editor/settings stay available; profile selection while disabled changes selection only; runtime-only state resets to enabled on restart (§6).
- **D5 — Launch button:** performs real execution; a separate Preview action inspects steps without running; no simulated success (§5 / anti-pattern table).
- **D6 — Profile switching:** rail = load draft for editing; Home = select only (explicit Apply or Launch & Arrange); tray = apply immediately when enabled, select-only when disabled (§2a / §4.5).

### Other decided items
- **Multi-monitor support — NOT in scope.** Primary work area only, matching the AHK script. No multi-monitor work should be planned or implemented.
- **Profile-specific assignments — CONFIRMED and already supported by the data model** (§1a). No data-model rewrite needed.
- **Edit / Save / Apply / Disable are separate verbs** (§1). No combined "Save & Apply" in the redesigned UI.

### Remaining genuinely unresolved technical questions (not product decisions)
1. **Same app in multiple zones within one profile** (§3b): backend returns a single HWND per exe/path — document the limitation and warn in the editor; proper multi-window matching is a deferred capability. No product decision needed — the safe behavior is defined.
2. **Responsive breakpoints vs minimum window size** (GUI architecture §2.3): **resolved** — minimum window size lowered to 700pt so the 700–799pt breakpoint is reachable. No decision needed.
3. **Repository branches** — **resolved (2026-11-21):** consolidated to a single `main` branch locally and remotely. The empty legacy `main` was deleted, `master` was renamed to `main` (history preserved), and GitHub's default branch set to `main`. No history rewrite or force-push.
